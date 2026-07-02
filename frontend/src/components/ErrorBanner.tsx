import { AlertCircle, X } from "lucide-react";
import { Button } from "./ui/Button";

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
        <Button
          className="db-error__close"
          aria-label="关闭"
          onClick={onClose}
        >
          <X size={14} />
        </Button>
      )}
    </div>
  );
}
