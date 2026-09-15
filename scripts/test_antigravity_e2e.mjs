import { execSync } from 'child_process';

const BASE_URL = 'http://127.0.0.1:8420';
const WS_BASE_URL = 'ws://127.0.0.1:8420';

async function main() {
  console.log('[ANTIGRAVITY E2E] Checking server health...');
  const healthRes = await fetch(`${BASE_URL}/api/health`);
  if (!healthRes.ok) {
    throw new Error(`Health check failed: ${healthRes.status}`);
  }
  const healthJson = await healthRes.json();
  console.log('[ANTIGRAVITY E2E] Health OK:', healthJson);

  // 1. Start workspace with ANTIGRAVITY + AUTO
  console.log('[ANTIGRAVITY E2E] Starting workspace with ANTIGRAVITY native stream-json...');
  const startReq = {
    name: `agentos-agy-native-${Date.now()}`,
    repos: [
      {
        repo_id: '9a1c7b67-f448-4278-9304-23252500c185',
        target_branch: 'main'
      }
    ],
    executor_config: {
      executor: 'ANTIGRAVITY',
      model_id: 'gemini-3.8-flash-high',
      permission_policy: 'AUTO'
    },
    prompt: "Por favor ejecuta el comando bash 'git rev-parse HEAD' y responde exactamente el SHA. No hagas nada más."
  };

  const startRes = await fetch(`${BASE_URL}/api/workspaces/start`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(startReq)
  });

  if (!startRes.ok) {
    const errorText = await startRes.text();
    throw new Error(`Start workspace failed (${startRes.status}): ${errorText}`);
  }

  const startJson = await startRes.json();
  const workspaceId = startJson.data.workspace.id;
  const executionProcessId = startJson.data.execution_process.id;
  console.log(`[ANTIGRAVITY E2E] Workspace created: ${workspaceId}`);
  console.log(`[ANTIGRAVITY E2E] Execution process started: ${executionProcessId}`);

  // Query session for workspace
  const sessionsRes = await fetch(`${BASE_URL}/api/sessions?workspace_id=${workspaceId}`);
  const sessionsJson = await sessionsRes.json();
  const sessionId = sessionsJson.data[0]?.id;
  console.log(`[ANTIGRAVITY E2E] Session resolved: ${sessionId}`);

  // 2. Capture logs via WebSocket
  const capturedLogs = [];
  const logsWs = new WebSocket(`${WS_BASE_URL}/api/execution-processes/${executionProcessId}/raw-logs/ws`);

  logsWs.onmessage = (event) => {
    try {
      const line = event.data.toString();
      capturedLogs.push(line);
      const parsed = JSON.parse(line);
      if (parsed.event === 'init') {
        console.log(`[ANTIGRAVITY E2E][Log] init event received, conversation_id=${parsed.conversation_id}`);
      } else if (parsed.event === 'step_update') {
        const su = parsed.step_update;
        if (su.step_type === 'tool') {
          console.log(`[ANTIGRAVITY E2E][Log] Tool ${su.tool_name}: state=${su.state}`);
        } else if (su.step_type === 'agent_response' && su.text_delta) {
          process.stdout.write(`[ANTIGRAVITY E2E][Stream] ${su.text_delta}`);
        }
      } else if (parsed.event === 'result') {
        console.log(`\n[ANTIGRAVITY E2E][Log] Result: status=${parsed.result?.status}, turns=${parsed.result?.num_turns}`);
      }
    } catch (e) {
      // plain text log line
    }
  };

  // 3. Poll Turn 1 execution process status until completed
  console.log('[ANTIGRAVITY E2E] Awaiting Turn 1 completion...');
  let finalStatus1 = null;
  const startTime = Date.now();
  while (Date.now() - startTime < 120000) {
    await new Promise(r => setTimeout(r, 2000));
    const epRes = await fetch(`${BASE_URL}/api/execution-processes/${executionProcessId}`);
    if (epRes.ok) {
      const epJson = await epRes.json();
      finalStatus1 = epJson.data;
      if (finalStatus1.status === 'completed' || finalStatus1.status === 'failed' || finalStatus1.status === 'killed') {
        console.log(`[ANTIGRAVITY E2E] Turn 1 process ended with status=${finalStatus1.status}, exit_code=${finalStatus1.exit_code}`);
        break;
      }
    }
  }
  logsWs.close();

  if (!finalStatus1 || finalStatus1.status !== 'completed' || finalStatus1.exit_code !== 0) {
    throw new Error(`Turn 1 failed or did not complete cleanly: ${JSON.stringify(finalStatus1)}`);
  }

  // 4. Test Follow-Up Turn (Turn 2) to verify session continuation (--conversation)
  console.log('\n[ANTIGRAVITY E2E] Starting Turn 2 (Follow-up) to test conversation persistence...');
  const followUpReq = {
    prompt: "¿Cuál fue el SHA que me diste en tu respuesta anterior? Responde únicamente con el SHA.",
    executor_config: {
      executor: 'ANTIGRAVITY',
      model_id: 'gemini-3.8-flash-high',
      permission_policy: 'AUTO'
    }
  };

  const followUpRes = await fetch(`${BASE_URL}/api/sessions/${sessionId}/follow-up`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(followUpReq)
  });

  if (!followUpRes.ok) {
    const errText = await followUpRes.text();
    throw new Error(`Follow-up failed (${followUpRes.status}): ${errText}`);
  }

  const followUpJson = await followUpRes.json();
  const followUpProcessId = followUpJson.data.id;
  console.log(`[ANTIGRAVITY E2E] Follow-up execution process started: ${followUpProcessId}`);

  let finalStatus2 = null;
  const startTime2 = Date.now();
  while (Date.now() - startTime2 < 120000) {
    await new Promise(r => setTimeout(r, 2000));
    const epRes = await fetch(`${BASE_URL}/api/execution-processes/${followUpProcessId}`);
    if (epRes.ok) {
      const epJson = await epRes.json();
      finalStatus2 = epJson.data;
      if (finalStatus2.status === 'completed' || finalStatus2.status === 'failed' || finalStatus2.status === 'killed') {
        console.log(`[ANTIGRAVITY E2E] Turn 2 process ended with status=${finalStatus2.status}, exit_code=${finalStatus2.exit_code}`);
        break;
      }
    }
  }

  if (!finalStatus2 || finalStatus2.status !== 'completed' || finalStatus2.exit_code !== 0) {
    throw new Error(`Turn 2 failed or did not complete cleanly: ${JSON.stringify(finalStatus2)}`);
  }

  console.log('\n================== ANTIGRAVITY E2E VERIFICATION RESULTS ==================');
  console.log('1. Workspace ID:', workspaceId);
  console.log('2. Session ID:', sessionId);
  console.log('3. Turn 1 Process ID:', executionProcessId, 'Status:', finalStatus1.status, 'Exit Code:', finalStatus1.exit_code);
  console.log('4. Turn 2 Follow-Up Process ID:', followUpProcessId, 'Status:', finalStatus2.status, 'Exit Code:', finalStatus2.exit_code);
  console.log('5. Integration Type: Native agy stream-json (Zero npm agy-acp dependency)');
  console.log('=========================================================================\n');
}

main().catch(err => {
  console.error('[ANTIGRAVITY E2E FATAL ERROR]', err);
  process.exit(1);
});
