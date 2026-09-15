import { execSync } from 'child_process';

const BASE_URL = 'http://127.0.0.1:8420';
const WS_BASE_URL = 'ws://127.0.0.1:8420';

async function main() {
  console.log('[E2E TEST] Checking server health...');
  const healthRes = await fetch(`${BASE_URL}/api/health`);
  if (!healthRes.ok) {
    throw new Error(`Health check failed: ${healthRes.status} ${healthRes.statusText}`);
  }
  const healthJson = await healthRes.json();
  console.log('[E2E TEST] Server health:', healthJson);

  // 1. Start workspace with supervised permission policy
  console.log('[E2E TEST] Starting workspace with CLAUDE_CODE + SUPERVISED...');
  const startReq = {
    name: `agentos-hitl-${Date.now()}`,
    repos: [
      {
        repo_id: '9a1c7b67-f448-4278-9304-23252500c185',
        target_branch: 'main'
      }
    ],
    executor_config: {
      executor: 'CLAUDE_CODE',
      permission_policy: 'SUPERVISED'
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
  console.log(`[E2E TEST] Workspace created: ${workspaceId}`);
  console.log(`[E2E TEST] Execution process started: ${executionProcessId}`);

  // 2. Set up WebSockets for raw-logs and approvals
  let approvalDetected = null;
  let approvalResponded = false;
  const capturedLogs = [];

  const logsWs = new WebSocket(`${WS_BASE_URL}/api/execution-processes/${executionProcessId}/raw-logs/ws`);
  const approvalsWs = new WebSocket(`${WS_BASE_URL}/api/approvals/stream/ws`);

  approvalsWs.onmessage = async (event) => {
    try {
      const msg = JSON.parse(event.data);
      console.log('[E2E TEST][Approvals WS] Event:', JSON.stringify(msg).slice(0, 160));
      // Check for approval creation patch
      if (msg.JsonPatch) {
        for (const op of msg.JsonPatch) {
          if ((op.op === 'add' || op.op === 'replace') && op.value && op.value.approval_id) {
            console.log(`[E2E TEST][Approvals WS] Detected pending approval: ${op.value.approval_id} for tool: ${op.value.tool_name}`);
            if (!approvalDetected) {
              approvalDetected = {
                id: op.value.approval_id,
                tool_name: op.value.tool_name,
                source: 'approvals_ws'
              };
              handleApproval(approvalDetected, workspaceId, executionProcessId);
            }
          }
        }
      }
    } catch (e) {
      console.error('[E2E TEST][Approvals WS] Parse error:', e);
    }
  };

  logsWs.onmessage = async (event) => {
    try {
      const line = event.data.toString();
      capturedLogs.push(line);
      const parsed = JSON.parse(line);
      if (parsed.type === 'approval_requested') {
        console.log(`[E2E TEST][Logs WS] Approval requested: id=${parsed.approval_id}, tool=${parsed.tool_name}`);
        if (!approvalDetected) {
          approvalDetected = {
            id: parsed.approval_id,
            tool_name: parsed.tool_name,
            source: 'logs_ws'
          };
          handleApproval(approvalDetected, workspaceId, executionProcessId);
        }
      } else if (parsed.type === 'assistant') {
        console.log(`[E2E TEST][Logs WS] Assistant: model=${parsed.message?.model || parsed.model}`);
      } else if (parsed.type === 'result') {
        console.log(`[E2E TEST][Logs WS] Result event: subtype=${parsed.subtype}, duration_ms=${parsed.duration_ms}`);
      }
    } catch (e) {
      // not JSON or parsing issue
    }
  };

  async function handleApproval(approval, wsId, epId) {
    if (approvalResponded) return;
    approvalResponded = true;
    console.log(`[E2E TEST] Pausing 2.5s while agent is BLOCKED to verify status...`);
    await new Promise(r => setTimeout(r, 2500));

    // Check workspace summary for has_pending_approval
    try {
      const sumRes = await fetch(`${BASE_URL}/api/workspaces/summaries`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ archived: false })
      });
      if (sumRes.ok) {
        const sumJson = await sumRes.json();
        const wsSummary = sumJson.data.summaries.find(s => s.workspace_id === wsId);
        console.log(`[E2E TEST] Workspace summary has_pending_approval: ${wsSummary?.has_pending_approval}`);
      }
    } catch (e) {
      console.warn('[E2E TEST] Failed to query workspace summary:', e.message);
    }

    // Now send human approval
    console.log(`[E2E TEST] Sending human APPROVAL for request: ${approval.id}`);
    const respondRes = await fetch(`${BASE_URL}/api/approvals/${approval.id}/respond`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        execution_process_id: epId,
        status: { status: 'approved' }
      })
    });

    if (respondRes.ok) {
      const respJson = await respondRes.json();
      console.log('[E2E TEST] Approval response accepted by server:', respJson);
    } else {
      const err = await respondRes.text();
      console.error(`[E2E TEST] Approval response failed (${respondRes.status}): ${err}`);
    }
  }

  // 3. Poll execution process status until completed
  console.log('[E2E TEST] Awaiting process completion...');
  let finalStatus = null;
  const startTime = Date.now();
  while (Date.now() - startTime < 120000) { // 2 minute timeout
    await new Promise(r => setTimeout(r, 2000));
    const epRes = await fetch(`${BASE_URL}/api/execution-processes/${executionProcessId}`);
    if (epRes.ok) {
      const epJson = await epRes.json();
      finalStatus = epJson.data;
      if (finalStatus.status === 'completed' || finalStatus.status === 'failed' || finalStatus.status === 'killed') {
        console.log(`[E2E TEST] Process terminated with status=${finalStatus.status}, exit_code=${finalStatus.exit_code}`);
        break;
      }
    }
  }

  logsWs.close();
  approvalsWs.close();

  // 4. Validate results
  if (!finalStatus || finalStatus.status !== 'completed' || finalStatus.exit_code !== 0) {
    throw new Error(`Process failed or did not complete cleanly: ${JSON.stringify(finalStatus)}`);
  }

  console.log('\n================== E2E VERIFICATION RESULTS ==================');
  console.log('1. Workspace ID:', workspaceId);
  console.log('2. Execution Process ID:', executionProcessId);
  console.log('3. Status:', finalStatus.status);
  console.log('4. Exit Code:', finalStatus.exit_code);
  console.log('5. Approval ID captured and approved:', approvalDetected?.id);
  console.log('6. Approval Tool Name:', approvalDetected?.tool_name);

  // Check worktree git status
  try {
    const branches = execSync('git -C "C:\\Users\\iagui\\AI Projects\\AgentOS-Prueba\\agentos-e2e" branch -a').toString();
    const createdBranch = branches.split('\n').find(b => b.includes(workspaceId.slice(0, 4)));
    console.log(`7. Branch created: ${createdBranch ? createdBranch.trim() : 'not found'}`);
  } catch (e) {
    console.warn('Could not inspect branch:', e.message);
  }

  console.log('==============================================================\n');
}

main().catch(err => {
  console.error('[E2E TEST FATAL ERROR]', err);
  process.exit(1);
});
