use crate::adb;
use crate::device;
use crate::runner::{self, TaskMode};
use crate::sqlite_token;
use crate::types::{
    AppState, CheckStatus, EnvCheckItem, EnvReport, EventEmitter, OfferItem, SolariEvent, StepId,
};
use std::sync::Arc;
use tokio::process::Command;

pub const SUBSCRIPTIONS_URL: &str = "https://play.google.com/store/account/subscriptions";
pub const DISCORD_URL: &str = "https://discord.gg/er49sMEQ48";

fn make_check(id: &str, label: &str, status: CheckStatus, detail: Option<String>) -> EnvCheckItem {
    EnvCheckItem {
        id: id.to_string(),
        label: label.to_string(),
        status,
        detail,
    }
}

fn update_and_emit(
    app: &EventEmitter,
    checks: &mut [EnvCheckItem],
    index: usize,
    status: CheckStatus,
    detail: Option<String>,
) {
    checks[index].status = status;
    checks[index].detail = detail;
    runner::emit_event(
        app,
        SolariEvent::EnvProgress {
            item: checks[index].clone(),
        },
    );
}

async fn resolve_context(state: &Arc<AppState>) -> Result<(String, String, String), String> {
    let adb_path = {
        let mut guard = state.adb_path.lock().await;
        if let Some(p) = guard.clone() {
            p
        } else {
            let found = adb::find_or_download_adb().await?;
            *guard = Some(found.clone());
            found
        }
    };

    let serial = {
        let mut guard = state.serial.lock().await;
        if let Some(s) = guard.clone() {
            s
        } else {
            let connected = adb::connect_emulator(&adb_path).await?;
            *guard = Some(connected.clone());
            connected
        }
    };

    let abi = {
        let mut guard = state.abi.lock().await;
        if let Some(a) = guard.clone() {
            a
        } else {
            let detected = device::get_abi(&adb_path, &serial).await?;
            *guard = Some(detected.clone());
            detected
        }
    };

    Ok((adb_path, serial, abi))
}

pub async fn check_environment(
    app: EventEmitter,
    state: Arc<AppState>,
) -> Result<EnvReport, String> {
    let mut checks = vec![
        make_check("adb", "ADB (Android Debug Bridge)", CheckStatus::Pending, None),
        make_check("emulator", "Emulador Conectado", CheckStatus::Pending, None),
        make_check("root", "Permissão Root", CheckStatus::Pending, None),
        make_check("network", "Rede & Proxy Limpos", CheckStatus::Pending, None),
        make_check("play_store", "Google Play Store", CheckStatus::Pending, None),
        make_check("google_one", "Google One Instalado", CheckStatus::Pending, None),
        make_check(
            "runner",
            "Motor Android (frida-inject + módulo)",
            CheckStatus::Pending,
            None,
        ),
    ];

    // 1. ADB
    update_and_emit(&app, &mut checks, 0, CheckStatus::Running, None);
    let adb_path = match adb::find_or_download_adb().await {
        Ok(p) => {
            *state.adb_path.lock().await = Some(p.clone());
            update_and_emit(&app, &mut checks, 0, CheckStatus::Ok, Some(p.clone()));
            runner::emit_log(&app, "ok", format!("ADB pronto: {}", p));
            p
        }
        Err(e) => {
            update_and_emit(&app, &mut checks, 0, CheckStatus::Error, Some(e.clone()));
            runner::emit_log(&app, "error", e);
            return Ok(EnvReport {
                ready: false,
                adb_path: None,
                serial: None,
                abi: None,
                checks,
            });
        }
    };
    let adb_path_opt = Some(adb_path.clone());

    // 2. Emulador conectado
    update_and_emit(&app, &mut checks, 1, CheckStatus::Running, None);
    let serial = match adb::connect_emulator(&adb_path).await {
        Ok(s) => {
            *state.serial.lock().await = Some(s.clone());
            update_and_emit(&app, &mut checks, 1, CheckStatus::Ok, Some(s.clone()));
            runner::emit_log(&app, "ok", format!("Emulador conectado: {}", s));
            s
        }
        Err(e) => {
            update_and_emit(&app, &mut checks, 1, CheckStatus::Error, Some(e.clone()));
            runner::emit_log(&app, "error", e);
            return Ok(EnvReport {
                ready: false,
                adb_path: adb_path_opt,
                serial: None,
                abi: None,
                checks,
            });
        }
    };
    let serial_opt = Some(serial.clone());

    // 3. Root
    update_and_emit(&app, &mut checks, 2, CheckStatus::Running, None);
    if let Err(e) = device::ensure_root(&adb_path, &serial).await {
        update_and_emit(&app, &mut checks, 2, CheckStatus::Error, Some(e.clone()));
        runner::emit_log(&app, "error", e);
        return Ok(EnvReport {
            ready: false,
            adb_path: adb_path_opt,
            serial: serial_opt,
            abi: None,
            checks,
        });
    }
    update_and_emit(
        &app,
        &mut checks,
        2,
        CheckStatus::Ok,
        Some("uid=0 ativo".to_string()),
    );
    runner::emit_log(&app, "ok", "Permissão root verificada (uid=0).");

    // 4. Rede & Proxy
    update_and_emit(&app, &mut checks, 3, CheckStatus::Running, None);
    match device::ensure_network(&adb_path, &serial).await {
        Ok(actions) => {
            let detail = if actions.is_empty() {
                "Sem proxy ou VPN bloqueando".to_string()
            } else {
                actions.join("; ")
            };
            update_and_emit(&app, &mut checks, 3, CheckStatus::Ok, Some(detail.clone()));
            runner::emit_log(&app, "ok", format!("Rede limpa: {}", detail));
        }
        Err(e) => {
            update_and_emit(&app, &mut checks, 3, CheckStatus::Warning, Some(e.clone()));
            runner::emit_log(&app, "warn", format!("Aviso ao limpar rede: {}", e));
        }
    }

    // 5. Google Play Store
    update_and_emit(&app, &mut checks, 4, CheckStatus::Running, None);
    if let Err(e) = device::ensure_vending(&adb_path, &serial).await {
        update_and_emit(&app, &mut checks, 4, CheckStatus::Error, Some(e.clone()));
        runner::emit_log(&app, "error", e);
        return Ok(EnvReport {
            ready: false,
            adb_path: adb_path_opt,
            serial: serial_opt,
            abi: None,
            checks,
        });
    }
    update_and_emit(
        &app,
        &mut checks,
        4,
        CheckStatus::Ok,
        Some("com.android.vending em execução".to_string()),
    );
    runner::emit_log(&app, "ok", "Google Play Store em execução.");

    // 6. Google One
    update_and_emit(&app, &mut checks, 5, CheckStatus::Running, None);
    if let Err(e) = device::check_google_one(&adb_path, &serial).await {
        update_and_emit(&app, &mut checks, 5, CheckStatus::Error, Some(e.clone()));
        runner::emit_log(&app, "error", e);
        return Ok(EnvReport {
            ready: false,
            adb_path: adb_path_opt,
            serial: serial_opt,
            abi: None,
            checks,
        });
    }
    update_and_emit(
        &app,
        &mut checks,
        5,
        CheckStatus::Ok,
        Some("com.google.android.apps.subscriptions.red instalado".to_string()),
    );
    runner::emit_log(&app, "ok", "Google One instalado.");

    // 7. Motor Android (ABI + frida-inject + module_521.apk)
    update_and_emit(&app, &mut checks, 6, CheckStatus::Running, None);
    let abi = match device::get_abi(&adb_path, &serial).await {
        Ok(a) => {
            *state.abi.lock().await = Some(a.clone());
            a
        }
        Err(e) => {
            update_and_emit(&app, &mut checks, 6, CheckStatus::Error, Some(e.clone()));
            runner::emit_log(&app, "error", e);
            return Ok(EnvReport {
                ready: false,
                adb_path: adb_path_opt,
                serial: serial_opt,
                abi: None,
                checks,
            });
        }
    };
    let abi_opt = Some(abi.clone());

    if let Err(e) = runner::prepare_device_binaries(&adb_path, &serial, &abi, Some(&app)).await {
        update_and_emit(&app, &mut checks, 6, CheckStatus::Error, Some(e.clone()));
        runner::emit_log(&app, "error", e);
        return Ok(EnvReport {
            ready: false,
            adb_path: adb_path_opt,
            serial: serial_opt,
            abi: abi_opt,
            checks,
        });
    }

    update_and_emit(
        &app,
        &mut checks,
        6,
        CheckStatus::Ok,
        Some(format!("ABI {} • frida-inject {} pronto", abi, runner::FRIDA_VERSION)),
    );
    runner::emit_log(&app, "ok", format!("Motor Android pronto (ABI {}).", abi));

    Ok(EnvReport {
        ready: true,
        adb_path: adb_path_opt,
        serial: serial_opt,
        abi: abi_opt,
        checks,
    })
}

pub async fn list_offers(
    app: EventEmitter,
    state: Arc<AppState>,
) -> Result<Vec<OfferItem>, String> {
    let (adb_path, serial, abi) = resolve_context(&state).await?;
    runner::emit_log(&app, "info", "Consultando ofertas disponíveis na conta...");

    runner::run_billing_task(
        &app,
        state.active_child_pid.clone(),
        &adb_path,
        &serial,
        &abi,
        TaskMode::ListOffers,
    )
    .await
}

pub async fn start_step(
    app: EventEmitter,
    state: Arc<AppState>,
    step_id: StepId,
) -> Result<(), String> {
    let (adb_path, serial, abi) = resolve_context(&state).await?;

    let old_token = if step_id.requires_token() {
        runner::emit_log(
            &app,
            "info",
            "Extraindo purchaseToken vigente do library.db...",
        );
        match sqlite_token::read_purchase_token(&adb_path, &serial, "subs_variant").await {
            Ok(info) => {
                runner::emit_log(
                    &app,
                    "ok",
                    format!(
                        "purchaseToken localizado (conta: {}, status: {}).",
                        info.account,
                        if info.is_valid { "ativo" } else { "expirado" }
                    ),
                );
                info.token
            }
            Err(e) => {
                runner::emit_log(
                    &app,
                    "warn",
                    format!(
                        "{} Usando fallback direto via BillingClient (queryPurchasesAsync)...",
                        e
                    ),
                );
                String::new()
            }
        }
    } else {
        String::new()
    };

    runner::emit_log(
        &app,
        "info",
        format!("Executando etapa {:?} (oferta {})...", step_id, step_id.offer_id()),
    );

    runner::run_billing_task(
        &app,
        state.active_child_pid.clone(),
        &adb_path,
        &serial,
        &abi,
        TaskMode::ExecuteStep { step_id, old_token },
    )
    .await?;

    Ok(())
}

pub async fn cancel_active_task(
    app: EventEmitter,
    state: Arc<AppState>,
) -> Result<(), String> {
    let pid = state.active_child_pid.lock().await.take();
    if let Some(p) = pid {
        #[cfg(windows)]
        {
            let mut cmd = Command::new("taskkill");
            adb::configure_command(&mut cmd);
            cmd.args(["/F", "/T", "/PID", &p.to_string()]);
            let _ = cmd.output().await;
        }
        #[cfg(not(windows))]
        {
            let mut cmd = Command::new("kill");
            cmd.args(["-9", &p.to_string()]);
            let _ = cmd.output().await;
        }
        runner::emit_log(&app, "warn", "Tarefa ativa cancelada pelo usuário.");
    }
    runner::emit_event(&app, SolariEvent::Cancelled);
    Ok(())
}

pub fn open_external_url(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        let mut cmd = std::process::Command::new("cmd");
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
        cmd.args(["/C", "start", "", url]);
        cmd.spawn()
            .map_err(|e| format!("Falha ao abrir navegador: {}", e))?;
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Falha ao abrir navegador: {}", e))?;
        return Ok(());
    }
}
