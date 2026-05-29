import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { ErrorBoundary } from "../components/ErrorBoundary";

function ThrowingComponent() {
  // eslint-disable-next-line no-constant-condition
  if (true) throw new Error("Test error");
  return null;
}

function WorkingComponent() {
  return <div>Working content</div>;
}

describe("ErrorBoundary", () => {
  it("renders children when no error", () => {
    render(
      <ErrorBoundary>
        <WorkingComponent />
      </ErrorBoundary>,
    );
    expect(screen.getByText("Working content")).toBeInTheDocument();
  });

  it("renders error UI when child throws", () => {
    render(
      <ErrorBoundary name="TestComponent">
        <ThrowingComponent />
      </ErrorBoundary>,
    );
    expect(screen.getByText("Something went wrong")).toBeInTheDocument();
    // Error text is split across elements: "Error", ": ", "Test error"
    const errorDiv = screen.getByText("Something went wrong").closest("div");
    expect(errorDiv?.textContent).toContain("Test error");
  });

  it("renders reload button", () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent />
      </ErrorBoundary>,
    );
    expect(screen.getByText("Reload")).toBeInTheDocument();
  });

  it("renders hard refresh button", () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent />
      </ErrorBoundary>,
    );
    expect(screen.getByText("Hard Refresh")).toBeInTheDocument();
  });
});
