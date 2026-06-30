import { AlertCircle, X } from "lucide-react";

interface ErrorBannerProps {
  message: string | null;
  onClose?: () => void;
}

export function ErrorBanner({ message, onClose }: ErrorBannerProps) {
  if (!message) return null;
  return (
    <div className="db-error" role="alert">
      <AlertCircle />
      <span style={{ flex: 1 }}>{message}</span>
      {onClose && (
        <button
          type="button"
          className="db-error__close"
          aria-label="关闭"
          onClick={onClose}
        >
          <X size={14} />
        </button>
      )}
    </div>
  );
}
