import { cn } from "../../lib/utils";

interface SpinnerProps {
  size?: "sm" | "md" | "lg";
  className?: string;
}

export function Spinner({ size = "md", className }: SpinnerProps) {
  const sizeClass =
    size === "sm" ? "w-4 h-4" : size === "lg" ? "w-8 h-8" : "w-5 h-5";
  return (
    <span
      className={cn(
        "inline-block border-2 border-primary/25 border-t-primary rounded-full animate-spin",
        sizeClass,
        className,
      )}
    />
  );
}
