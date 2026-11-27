import { motion } from "motion/react";
import { Check, Cpu, Eye, Layers, Search, ShieldCheck } from "lucide-react";
import aureonLogo from "@/assets/aureon-logo.png";
import { cn } from "@/lib/utils";

export type WizardStep = 1 | 2 | 3 | 4;

interface StepMeta {
  step: WizardStep;
  title: string;
  shortLabel: string;
  icon: React.ComponentType<{ className?: string }>;
}

const WIZARD_STEPS: StepMeta[] = [
  { step: 1, title: "Diagnóstico do Emulador", shortLabel: "Diagnóstico", icon: Cpu },
  { step: 2, title: "Consulta de Ofertas", shortLabel: "Ofertas", icon: Search },
  { step: 3, title: "Assistente de Acumulação", shortLabel: "Acumulação", icon: Layers },
  { step: 4, title: "Finalização Segura", shortLabel: "Finalização", icon: ShieldCheck },
];

interface StepperHeaderProps {
  currentStep: WizardStep;
  maxUnlockedStep: WizardStep;
  completedWizardSteps: Record<WizardStep, boolean>;
  onSelectStep: (step: WizardStep) => void;
  isTauri: boolean;
  previewEnabled: boolean;
  onTogglePreview: () => void;
}

export function StepperHeader({
  currentStep,
  maxUnlockedStep,
  completedWizardSteps,
  onSelectStep,
  isTauri,
  previewEnabled,
  onTogglePreview,
}: StepperHeaderProps) {
  return (
    <header className="aur-header">
      <div className="aur-masthead">
        <div className="flex items-center gap-3">
          <span className="aur-mark">
            <img src={aureonLogo} alt="Auréon" className="aur-mark-img" />
          </span>
          <div>
            <h1 className="text-2xl font-semibold leading-none tracking-[-.055em]">Auréon</h1>
            <p className="mt-1 text-xs text-muted-foreground">Assistente Google AI Pro</p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {!isTauri && (
            <button
              type="button"
              onClick={onTogglePreview}
              className={cn(
                "aur-button aur-button-secondary inline-flex min-h-9 items-center gap-2 px-3 text-xs font-medium",
                previewEnabled && "text-primary"
              )}
              aria-pressed={previewEnabled}
              title="Alternar dados simulados"
            >
              <Eye className="h-3.5 w-3.5" />
              <span>{previewEnabled ? "Simulação ativa" : "Simulação desativada"}</span>
            </button>
          )}
        </div>
      </div>

      <div className="aur-rail">
        <nav aria-label="Etapas do assistente" className="aur-nav">
          {WIZARD_STEPS.map(({ step, title, shortLabel, icon: Icon }) => {
            const isActive = currentStep === step;
            const isDone = completedWizardSteps[step];
            const isAccessible = step <= maxUnlockedStep || (previewEnabled && !isTauri);

            return (
              <button
                key={step}
                type="button"
                disabled={!isAccessible}
                aria-current={isActive ? "step" : undefined}
                title={title}
                onClick={() => isAccessible && onSelectStep(step)}
                className={cn(
                  "aur-nav-item",
                  isActive && "is-current",
                  isDone && "is-complete",
                )}
              >
                {isActive && (
                  <motion.span
                    layoutId="active-step-pill"
                    transition={{ type: "spring", stiffness: 320, damping: 34 }}
                    className="aur-nav-glass"
                  />
                )}
                <span className="aur-nav-icon">
                  {isDone ? <Check className="h-3.5 w-3.5" /> : <Icon className="h-3.5 w-3.5" />}
                </span>
                <span className="aur-nav-label">{shortLabel}</span>
              </button>
            );
          })}
        </nav>
        <p className="aur-rail-caption">Etapa {currentStep} de {WIZARD_STEPS.length}</p>
      </div>
    </header>
  );
}
