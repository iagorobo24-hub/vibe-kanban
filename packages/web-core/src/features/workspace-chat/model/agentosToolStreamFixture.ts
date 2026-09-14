import {
  BaseCodingAgent,
  PermissionPolicy,
  ExecutionProcessStatus,
  type ExecutionProcess,
  type NormalizedEntry,
  type PatchType,
} from 'shared/types';
import {
  AGENTOS_TOOL_FIXTURE_PROCESS_ID,
  AGENTOS_TOOL_FIXTURE_SESSION_ID,
  AGENTOS_APPROVAL_FIXTURE_ID,
  isAgentOSQaFixtureEnabled,
} from '@/shared/lib/agentOSQaFixtures';
import type {
  ConversationTimelineSource,
  ExecutionProcessState,
  PatchTypeWithKey,
} from '@/shared/hooks/useConversationHistory/types';

const FIXTURE_PROCESS_ID = AGENTOS_TOOL_FIXTURE_PROCESS_ID;
const FIXTURE_SESSION_ID = AGENTOS_TOOL_FIXTURE_SESSION_ID;
const FIXTURE_TIMESTAMP = '2026-09-14T10:00:00.000Z';

export function isAgentOSToolStreamFixtureEnabled(): boolean {
  return (
    isAgentOSQaFixtureEnabled('tool-stream') ||
    isAgentOSQaFixtureEnabled('approval')
  );
}

const fixtureProcess: ExecutionProcess = {
  id: FIXTURE_PROCESS_ID,
  session_id: FIXTURE_SESSION_ID,
  run_reason: 'codingagent',
  executor_action: {
    typ: {
      type: 'CodingAgentInitialRequest',
      prompt: 'Revisa el flujo de autenticación y prepara un resumen.',
      executor_config: {
        executor: BaseCodingAgent.OPENCODE,
        variant: 'Build',
        model_id: 'opencode/zen-free',
        agent_id: null,
        reasoning_id: 'high',
        permission_policy: PermissionPolicy.AUTO,
      },
      working_dir: null,
    },
    next_action: null,
  },
  status: isAgentOSQaFixtureEnabled('approval')
    ? ExecutionProcessStatus.running
    : ExecutionProcessStatus.completed,
  exit_code: 0n,
  dropped: false,
  started_at: FIXTURE_TIMESTAMP,
  completed_at: isAgentOSQaFixtureEnabled('approval')
    ? null
    : '2026-09-14T10:00:08.000Z',
  created_at: FIXTURE_TIMESTAMP,
  updated_at: '2026-09-14T10:00:08.000Z',
};

function normalizedEntry(
  index: number,
  content: NormalizedEntry['content'],
  entryType: NormalizedEntry['entry_type']
): PatchTypeWithKey {
  const patch: PatchType = {
    type: 'NORMALIZED_ENTRY',
    content: {
      content,
      entry_type: entryType,
      timestamp: FIXTURE_TIMESTAMP,
    },
  };

  return {
    ...patch,
    patchKey: `${FIXTURE_PROCESS_ID}:${index}`,
    executionProcessId: FIXTURE_PROCESS_ID,
  };
}

function buildFixtureEntries(): PatchTypeWithKey[] {
  if (isAgentOSQaFixtureEnabled('approval')) {
    return [
      normalizedEntry(
        0,
        'Analiza este comando y espera confirmación antes de ejecutarlo.',
        { type: 'user_message' }
      ),
      normalizedEntry(
        1,
        'He preparado la operación. Necesito aprobación humana antes de continuar.',
        { type: 'assistant_message' }
      ),
      normalizedEntry(
        2,
        'Se solicita permiso para ejecutar una comprobación de solo lectura.',
        {
          type: 'tool_use',
          tool_name: 'shell',
          action_type: {
            action: 'command_run',
            command: 'pnpm test --filter auth',
            category: 'read',
            result: null,
          },
          status: {
            status: 'pending_approval',
            approval_id: AGENTOS_APPROVAL_FIXTURE_ID,
          },
        }
      ),
    ];
  }

  return [
    normalizedEntry(
      0,
      'Revisa el flujo de autenticación y prepara un resumen.',
      { type: 'user_message' }
    ),
    normalizedEntry(
      1,
      'Voy a revisar primero la implementación de sesión, buscar los puntos de entrada y comprobar los tests relacionados.',
      { type: 'assistant_message' }
    ),
    normalizedEntry(2, 'src/auth/session.ts', {
      type: 'tool_use',
      tool_name: 'read_file',
      action_type: { action: 'file_read', path: 'src/auth/session.ts' },
      status: { status: 'success' },
    }),
    normalizedEntry(
      3,
      '3 coincidencias encontradas en rutas de autenticación.',
      {
        type: 'tool_use',
        tool_name: 'rg',
        action_type: { action: 'search', query: 'remote_auth_unavailable' },
        status: { status: 'success' },
      }
    ),
    normalizedEntry(
      4,
      'Los tests del flujo de sesión terminan correctamente.',
      {
        type: 'tool_use',
        tool_name: 'shell',
        action_type: {
          action: 'command_run',
          command: 'pnpm test --filter auth',
          category: 'read',
          result: {
            output: 'auth/session.test.ts · 8 passed',
            exit_status: { type: 'exit_code', code: 0 },
          },
        },
        status: { status: 'success' },
      }
    ),
    normalizedEntry(5, 'Configuración del proveedor cargada sin secretos.', {
      type: 'tool_use',
      tool_name: 'filesystem.read',
      action_type: {
        action: 'tool',
        tool_name: 'filesystem.read',
        arguments: { path: 'config/auth-policy.json' },
        result: {
          type: { type: 'json' },
          value: { provider: 'local', requires_auth: false },
        },
      },
      status: { status: 'success' },
    }),
    normalizedEntry(
      6,
      'El flujo mantiene la separación entre sesión local y autenticación remota. No se han modificado archivos.',
      { type: 'assistant_message' }
    ),
  ];
}

export function createAgentOSToolStreamFixture(): ConversationTimelineSource {
  const entries = buildFixtureEntries();
  const executionProcessState: ExecutionProcessState = {
    executionProcess: {
      id: fixtureProcess.id,
      created_at: fixtureProcess.created_at,
      updated_at: fixtureProcess.updated_at,
      executor_action: fixtureProcess.executor_action,
    },
    entries,
  };

  return {
    executionProcessState: {
      [fixtureProcess.id]: executionProcessState,
    },
    liveExecutionProcesses: [fixtureProcess],
  };
}
