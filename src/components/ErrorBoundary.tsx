import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  /** Optional name to identify which part of the app errored. */
  name?: string;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
  reportSent: boolean;
  reportFailed: boolean;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
      reportSent: false,
      reportFailed: false,
    };
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    this.setState({ error, errorInfo });
    console.error("[ErrorBoundary] Caught:", error, errorInfo);

    // Fire-and-forget: send the error report to the backend log.
    this.sendErrorReport(error, errorInfo);
  }

  private async sendErrorReport(
    error: Error,
    errorInfo: ErrorInfo,
  ): Promise<void> {
    try {
      // Use the bridge's reportError wrapper (handles Tauri readiness check)
      const { reportError } = await import("../lib/tauriBridge");
      const stack = [error.stack, errorInfo.componentStack]
        .filter(Boolean)
        .join("\n\n");
      await reportError({
        message: `${error.name}: ${error.message}`,
        stack: stack || null,
        component: this.props.name ?? null,
        timestamp: new Date().toISOString(),
      });
      this.setState({ reportSent: true });
    } catch {
      // Tauri bridge not available or command failed — that's OK.
      this.setState({ reportFailed: true });
    }
  }

  handleReload = () => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
      reportSent: false,
      reportFailed: false,
    });
  };

  render(): ReactNode {
    if (this.state.hasError) {
      const { error, errorInfo, reportSent, reportFailed } = this.state;

      return (
        <div className="flex flex-col items-center justify-center h-screen bg-background text-foreground p-6">
          <div className="max-w-[520px] w-full bg-card border border-border rounded-xl p-8 shadow-xl">
            <div className="w-14 h-14 rounded-full bg-destructive/10 flex items-center justify-center mb-5">
              <span className="text-[28px]">&#9888;</span>
            </div>

            <h1 className="mb-2 text-xl font-semibold text-destructive">
              Something went wrong
            </h1>

            <p className="mb-5 text-[13px] text-muted-foreground leading-relaxed">
              An unexpected error occurred while rendering the application. You
              can try reloading to recover.
            </p>

            {error && (
              <div className="mb-5 max-h-[160px] overflow-auto rounded-lg border border-destructive/20 bg-destructive/5 px-4 py-3">
                <p className="mb-1 text-xs font-medium text-destructive">
                  {error.name}: {error.message}
                </p>
                {errorInfo?.componentStack && (
                  <pre className="text-[11px] text-muted-foreground whitespace-pre-wrap break-words">
                    {errorInfo.componentStack.trim()}
                  </pre>
                )}
              </div>
            )}

            <div className="flex items-center gap-2.5">
              <button
                onClick={this.handleReload}
                className="rounded-lg border-0 bg-primary px-6 py-2.5 text-[13px] font-medium text-primary-foreground outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring cursor-pointer"
              >
                Reload
              </button>
              <button
                onClick={() => window.location.reload()}
                className="rounded-lg border border-border bg-transparent px-6 py-2.5 text-[13px] font-medium text-muted-foreground outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring cursor-pointer"
              >
                Hard Refresh
              </button>
              {reportSent && (
                <span className="text-xs text-success">
                  &#10003; Report sent
                </span>
              )}
              {reportFailed && (
                <span className="text-xs text-muted-foreground">
                  (offline — not reported)
                </span>
              )}
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;
