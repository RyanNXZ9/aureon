import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AnimatePresence, motion, MotionConfig, useReducedMotion } from "motion/react";
import { LogConsole, type LogEntry } from "@/components/LogConsole";
import { StepperHeader, type WizardStep } from "@/components/StepperHeader";
import { Step1Environment } from "@/components/steps/Step1Environment";
import { Step2Offers } from "@/components/steps/Step2Offers";
import { Step3Accumulation } from "@/components/steps/Step3Accumulation";
import { Step4Finish } from "@/components/steps/Step4Finish";
import { createPreviewController, DEFAULT_ENV_CHECKS } from "@/lib/preview";
import {
  cancelActiveTask,
  checkEnvironment,
  isTauriRuntime,
  listOffers,
  onAureonEvent,
  openSubscriptionsUrl,
  startStep,
} from "@/lib/tauri";
import type { EnvCheckItem, EnvReport, OfferItem, AureonEvent, StepId } from "@/types/aureon";

function formatNow(): string {
  return new Date().toLocaleTimeString("pt-BR", { hour12: false });
}

export function App() {
  const isTauri = isTauriRuntime();
  const reduceMotion = useReducedMotion();
  const [previewEnabled, setPreviewEnabled] = useState(!isTauri);

  const [currentStep, setCurrentStep] = useState<WizardStep>(1);
  const [maxUnlockedStep, setMaxUnlockedStep] = useState<WizardStep>(1);

  // Step 1 state
  const [envChecks, setEnvChecks] = useState<EnvCheckItem[]>(DEFAULT_ENV_CHECKS);
  const [envReport, setEnvReport] = useState<EnvReport | null>(null);
  const [isCheckingEnv, setIsCheckingEnv] = useState(false);
  const [step1Error, setStep1Error] = useState<string | null>(null);

  // Step 2 state
  const [offers, setOffers] = useState<OfferItem[]>([]);
  const [hasCheckedOffers, setHasCheckedOffers] = useState(false);
  const [isLoadingOffers, setIsLoadingOffers] = useState(false);
  const [step2Error, setStep2Error] = useState<string | null>(null);

  // Step 3 state
  const [completedSubSteps, setCompletedSubSteps] = useState<StepId[]>([]);
  const [activeStepId, setActiveStepId] = useState<StepId | null>(null);
  const [step3Status, setStep3Status] = useState<string | null>(null);
  const [step3Error, setStep3Error] = useState<string | null>(null);

  // Logs state
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const logSeqRef = useRef(1);

  const appendLog = useCallback(
    (level: LogEntry["level"], message: string, timestamp = formatNow()) => {
      setLogs((prev) => {
        const next: LogEntry = {
          id: logSeqRef.current++,
          level,
          message,
          timestamp,
        };
        return [...prev.slice(-299), next];
      });
    },
    []
  );

  const markSubStepCompleted = useCallback((stepId: StepId) => {
    setCompletedSubSteps((prev) => (prev.includes(stepId) ? prev : [...prev, stepId]));
    setActiveStepId((curr) => (curr === stepId ? null : curr));
  }, []);

  const handleAureonEvent = useCallback(
    (event: AureonEvent) => {
      switch (event.type) {
        case "log":
          appendLog(event.level, event.message, event.timestamp);
          break;

        case "env_progress":
          setEnvChecks((prev) =>
            prev.map((item) => (item.id === event.item.id ? event.item : item))
          );
          break;

        case "offer":
          setOffers((prev) => {
            const exists = prev.some(
              (o) => o.product === event.offer.product && o.offer_id === event.offer.offer_id
            );
            if (exists) {
              return prev.map((o) =>
                o.product === event.offer.product && o.offer_id === event.offer.offer_id
                  ? event.offer
                  : o
              );
            }
            return [...prev, event.offer];
          });
          break;

        case "chosen":
          appendLog(
            "info",
            `Oferta selecionada: ${event.offer_id} (${event.period}) em ${event.product}`
          );
          break;

        case "launched":
          setStep3Status(event.msg);
          setStep3Error(null);
          break;

        case "purchase_cb":
          if (event.code === 0) {
            markSubStepCompleted(event.step_id);
            setStep3Status(
              `Passo '${event.step_id}' confirmado (code = 0). Avançando para a próxima etapa.`
            );
            setStep3Error(null);
          } else {
            setActiveStepId(null);
            setStep3Status(null);
            setStep3Error(
              `Retorno da Play Store no passo '${event.step_id}' (código ${event.code}): ${event.msg}`
            );
          }
          break;

        case "step_done":
          if (event.success) {
            markSubStepCompleted(event.step_id);
            setStep3Status(event.message);
            setStep3Error(null);
          } else {
            setActiveStepId(null);
            setStep3Status(null);
            setStep3Error(event.message);
          }
          break;

        case "error":
          appendLog("error", event.message);
          setIsCheckingEnv(false);
          setIsLoadingOffers(false);
          setActiveStepId(null);
          setStep3Error(event.message);
          break;

        case "cancelled":
          setActiveStepId(null);
          setIsCheckingEnv(false);
          setIsLoadingOffers(false);
          setStep3Status("Tarefa cancelada pelo usuário.");
          break;
      }
    },
    [appendLog, markSubStepCompleted]
  );

  const previewController = useMemo(
    () => createPreviewController(handleAureonEvent),
    [handleAureonEvent]
  );

  useEffect(() => {
    if (!isTauri) {
      appendLog(
        "info",
        "Executando fora do runtime Tauri. Modo Preview ativado para simulação no navegador."
      );
      return;
    }

    let unlisten: (() => void) | undefined;
    let active = true;

    onAureonEvent(handleAureonEvent)
      .then((fn) => {
        if (active) {
          unlisten = fn;
        } else {
          fn();
        }
      })
      .catch((err) => {
        appendLog("error", `Falha ao registrar ouvinte de eventos: ${String(err)}`);
      });

    return () => {
      active = false;
      unlisten?.();
    };
  }, [appendLog, handleAureonEvent, isTauri]);

  const handleRunEnvCheck = async () => {
    setStep1Error(null);
    setEnvReport(null);
    setEnvChecks(DEFAULT_ENV_CHECKS);
    setIsCheckingEnv(true);

    try {
      const report = isTauri
        ? await checkEnvironment()
        : previewEnabled
        ? await previewController.checkEnvironment()
        : await checkEnvironment();

      setEnvReport(report);
      if (report.checks?.length) {
        setEnvChecks(report.checks);
      }
      if (report.ready) {
        setMaxUnlockedStep((prev) => (prev < 2 ? 2 : prev));
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg !== "PREVIEW_CANCELLED") {
        setStep1Error(msg);
        appendLog("error", `Erro no diagnóstico: ${msg}`);
      }
    } finally {
      setIsCheckingEnv(false);
    }
  };

  const handleCheckOffers = async () => {
    setStep2Error(null);
    setOffers([]);
    setIsLoadingOffers(true);

    try {
      const result = isTauri
        ? await listOffers()
        : previewEnabled
        ? await previewController.listOffers()
        : await listOffers();

      setOffers(result);
      setHasCheckedOffers(true);
      setMaxUnlockedStep((prev) => (prev < 3 ? 3 : prev));
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg !== "PREVIEW_CANCELLED") {
        setStep2Error(msg);
        appendLog("error", `Erro ao consultar ofertas: ${msg}`);
      }
    } finally {
      setIsLoadingOffers(false);
    }
  };

  const handleStartSubStep = async (stepId: StepId) => {
    if (activeStepId !== null) return;
    setStep3Error(null);
    setStep3Status(`Iniciando etapa '${stepId}'...`);
    setActiveStepId(stepId);

    try {
      if (isTauri) {
        await startStep(stepId);
      } else if (previewEnabled) {
        await previewController.startStep(stepId);
      } else {
        await startStep(stepId);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg !== "PREVIEW_CANCELLED") {
        setActiveStepId(null);
        setStep3Status(null);
        setStep3Error(msg);
        appendLog("error", `Erro ao executar '${stepId}': ${msg}`);
      }
    }
  };

  const handleCancelActiveTask = () => {
    // Libera a UI na hora: o backend pode estar travado num adb sem resposta.
    setActiveStepId(null);
    setStep3Status("Tarefa cancelada pelo usuário.");
    setStep3Error(null);

    const onError = (err: unknown) => {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg !== "PREVIEW_CANCELLED") {
        appendLog("error", `Erro ao cancelar tarefa: ${msg}`);
      }
    };

    if (isTauri) {
      void cancelActiveTask().catch(onError);
    } else if (previewEnabled) {
      void previewController.cancelActiveTask().catch(onError);
    }
  };

  const handleOpenSubscriptions = async () => {
    try {
      await openSubscriptionsUrl();
      appendLog("ok", "Página de assinaturas da Google Play aberta.");
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      appendLog("error", `Não foi possível abrir o link de assinaturas: ${msg}`);
    }
  };

  const handleResetWizard = () => {
    setCurrentStep(1);
    setMaxUnlockedStep(1);
    setEnvChecks(DEFAULT_ENV_CHECKS);
    setEnvReport(null);
    setStep1Error(null);
    setOffers([]);
    setHasCheckedOffers(false);
    setStep2Error(null);
    setCompletedSubSteps([]);
    setActiveStepId(null);
    setStep3Status(null);
    setStep3Error(null);
    appendLog("info", "Assistente reiniciado.");
  };

  const goToStep = (step: WizardStep) => {
    setMaxUnlockedStep((prev) => (step > prev ? step : prev));
    setCurrentStep(step);
  };

  const completedWizardSteps: Record<WizardStep, boolean> = {
    1: envReport?.ready === true,
    2: hasCheckedOffers && offers.length > 0,
    3: completedSubSteps.length > 0,
    4: false,
  };

  return (
    <MotionConfig reducedMotion="user">
    <div className="aur-app">
      <div className="aur-shell">
        <StepperHeader
          currentStep={currentStep}
          maxUnlockedStep={maxUnlockedStep}
          completedWizardSteps={completedWizardSteps}
          onSelectStep={setCurrentStep}
          isTauri={isTauri}
          previewEnabled={previewEnabled}
          onTogglePreview={() => setPreviewEnabled((v) => !v)}
        />

        <main className="aur-workspace" aria-label="Etapa atual">
          <AnimatePresence mode="wait">
            <motion.div
              key={currentStep}
              initial={{ opacity: 0, y: reduceMotion ? 0 : 9 }}
              animate={{ opacity: 1, y: 0, transition: { duration: reduceMotion ? 0 : 0.24, ease: [0.22, 1, 0.36, 1] } }}
              exit={{ opacity: 0, y: reduceMotion ? 0 : -4, transition: { duration: reduceMotion ? 0 : 0.1 } }}
            >
              {currentStep === 1 && (
                <Step1Environment
                  checks={envChecks}
                  report={envReport}
                  isChecking={isCheckingEnv}
                  errorMessage={step1Error}
                  onRunCheck={handleRunEnvCheck}
                  onNext={() => goToStep(2)}
                />
              )}

              {currentStep === 2 && (
                <Step2Offers
                  offers={offers}
                  hasChecked={hasCheckedOffers}
                  isLoading={isLoadingOffers}
                  errorMessage={step2Error}
                  onCheckOffers={handleCheckOffers}
                  onBack={() => setCurrentStep(1)}
                  onNext={() => goToStep(3)}
                />
              )}

              {currentStep === 3 && (
                <Step3Accumulation
                  completedSubSteps={completedSubSteps}
                  activeStepId={activeStepId}
                  statusMessage={step3Status}
                  errorMessage={step3Error}
                  onStartStep={handleStartSubStep}
                  onCancelTask={handleCancelActiveTask}
                  onBack={() => setCurrentStep(2)}
                  onNext={() => goToStep(4)}
                />
              )}

              {currentStep === 4 && (
                <Step4Finish
                  completedSubSteps={completedSubSteps}
                  onOpenSubscriptions={handleOpenSubscriptions}
                  onBack={() => setCurrentStep(3)}
                  onReset={handleResetWizard}
                />
              )}
            </motion.div>
          </AnimatePresence>
        </main>

        <LogConsole logs={logs} onClear={() => setLogs([])} />
        <footer className="aur-footer">
          <span>Interface por <strong>RyanNXZ9</strong></span>
          <a href="https://discord.gg/er49sMEQ48" target="_blank" rel="noopener noreferrer">
            Comunidade no Discord ↗
          </a>
        </footer>
      </div>
    </div>
    </MotionConfig>
  );
}

export default App;
