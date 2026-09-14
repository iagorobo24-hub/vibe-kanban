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
}

export function ProcessLogsViewerContent({
  logs,
  error,
  onRetry,
}: ProcessLogsViewerContentProps) {
  return (
    <VirtualizedProcessLogs
      logs={logs}
      error={error}
      searchQuery=""
      matchIndices={[]}
      currentMatchIndex={-1}
      onRetry={onRetry}
    />
  );
}

export default function ProcessLogsViewer({
  processId,
}: ProcessLogsViewerProps) {
  const { logs, error, retry } = useLogStream(processId);
  return <ProcessLogsViewerContent logs={logs} error={error} onRetry={retry} />;
}
