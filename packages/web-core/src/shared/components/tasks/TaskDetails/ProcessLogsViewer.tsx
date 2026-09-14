import { useLogStream } from '@/shared/hooks/useLogStream';
import {
  VirtualizedProcessLogs,
  type LogEntry,
} from '@/shared/components/VirtualizedProcessLogs';

interface ProcessLogsViewerProps {
  processId: string;
}

interface ProcessLogsViewerContentProps {
  logs: LogEntry[];
  error: string | null;
  onRetry?: () => void;
  isLoading?: boolean;
}

export function ProcessLogsViewerContent({
  logs,
  error,
  onRetry,
  isLoading = false,
}: ProcessLogsViewerContentProps) {
  return (
    <VirtualizedProcessLogs
      logs={logs}
      error={error}
      searchQuery=""
      matchIndices={[]}
      currentMatchIndex={-1}
      onRetry={onRetry}
      isLoading={isLoading}
    />
  );
}

export default function ProcessLogsViewer({
  processId,
}: ProcessLogsViewerProps) {
  const { logs, error, retry, isLoading } = useLogStream(processId);
  return (
    <ProcessLogsViewerContent
      logs={logs}
      error={error}
      onRetry={retry}
      isLoading={isLoading}
    />
  );
}
