import { Component, type ReactNode } from "react";

/** Un módulo que falla no debe tumbar la isla: se sustituye por `fallback` y se reintenta al cambiar `resetKey`. */
export class ErrorBoundary extends Component<{ children: ReactNode; fallback?: ReactNode; resetKey?: unknown }, { error: boolean }> {
  state = { error: false };

  static getDerivedStateFromError() {
    return { error: true };
  }

  componentDidCatch(error: unknown) {
    console.error("[isla] error en un módulo:", error);
  }

  componentDidUpdate(prev: { resetKey?: unknown }) {
    if (this.state.error && prev.resetKey !== this.props.resetKey) this.setState({ error: false });
  }

  render() {
    return this.state.error ? (this.props.fallback ?? null) : this.props.children;
  }
}
