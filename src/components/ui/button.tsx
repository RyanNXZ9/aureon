import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const aureonBtnVariants = cva(
  "aur-button inline-flex max-w-full items-center justify-center gap-2 text-center text-sm font-semibold",
  {
    variants: {
      variant: {
        default: "aur-button-primary",
        destructive: "aur-button-destructive",
        outline: "aur-button-secondary",
        secondary: "aur-button-secondary",
        ghost: "aur-button-quiet",
      },
      size: {
        default: "min-h-10 px-4 py-2",
        sm: "min-h-8 px-3 py-1.5 text-xs",
        lg: "min-h-11 px-5 py-2",
        icon: "h-9 w-9",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof aureonBtnVariants> {
  asChild?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return <Comp ref={ref} className={cn(aureonBtnVariants({ variant, size, className }))} {...props} />;
  }
);
Button.displayName = "Button";
