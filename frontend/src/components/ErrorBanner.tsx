import { Banner } from "@douyinfe/semi-ui";

interface ErrorBannerProps {
  message: string | null;
  onClose?: () => void;
}

export function ErrorBanner({ message, onClose }: ErrorBannerProps) {
  if (!message) return null;

  return (
    <Banner
      type="danger"
      description={message}
      closeIcon
      onClose={onClose}
      style={{ marginBottom: 16 }}
    />
  );
}
