import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  ArrowClockwiseIcon,
  CheckCircleIcon,
  CpuIcon,
  CurrencyDollarIcon,
  SpinnerIcon,
  TimerIcon,
  WarningCircleIcon,
  XCircleIcon,
} from '@phosphor-icons/react';
import { telemetryApi } from '@/shared/lib/api';
import type {
  ExecutionTelemetry,
  RecordExecutionTelemetry,
  TelemetrySummaryRow,
} from 'shared/types';
import { cn } from '@/shared/lib/utils';
import { PrimaryButton } from '@vibe/ui/components/PrimaryButton';
import { IconButton } from '@vibe/ui/components/IconButton';
import {
  SettingsCard,
  SettingsSelect,
} from './SettingsComponents';

export function TelemetrySettingsSection() {
  const [telemetryList, setTelemetryList] = useState<ExecutionTelemetry[]>([]);
  const [summaries, setSummaries] = useState<TelemetrySummaryRow[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [filterExecutor, setFilterExecutor] = useState<string>('all');
  const [filterOutcome, setFilterOutcome] = useState<string>('all');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editOutcome, setEditOutcome] = useState<string>('success');
  const [editNote, setEditNote] = useState<string>('');
  const [isSaving, setIsSaving] = useState(false);

  const loadData = useCallback(async () => {
    setIsLoading(true);
    try {
      const [list, sum] = await Promise.all([
        telemetryApi.list(),
        telemetryApi.summary(),
      ]);
      setTelemetryList(list);
      setSummaries(sum);
    } catch (err) {
      console.error('Failed to load telemetry data:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  // KPI Calculations
  const kpis = useMemo(() => {
    let totalCost = 0;
    let totalRuns = telemetryList.length;
    let totalDurationMs = 0;
    let durationCount = 0;
    let totalTokens = 0;

    for (const item of telemetryList) {
      if (item.cost_usd != null) totalCost += item.cost_usd;
      if (item.duration_ms != null) {
        totalDurationMs += Number(item.duration_ms);
        durationCount++;
      }
      if (item.total_tokens != null) {
        totalTokens += Number(item.total_tokens);
      }
    }

    const avgDurationSec =
      durationCount > 0 ? (totalDurationMs / durationCount / 1000).toFixed(1) : '0';

    return {
      totalCost: totalCost.toFixed(4),
      totalRuns,
      avgDurationSec,
      totalTokens:
        totalTokens >= 1_000_000
          ? `${(totalTokens / 1_000_000).toFixed(1)}M`
          : totalTokens >= 1_000
            ? `${(totalTokens / 1_000).toFixed(1)}K`
            : totalTokens.toString(),
    };
  }, [telemetryList]);

  // Filtered rows
  const filteredList = useMemo(() => {
    return telemetryList.filter((item) => {
      if (filterExecutor !== 'all' && item.executor !== filterExecutor) {
        return false;
      }
      if (filterOutcome !== 'all' && item.real_outcome !== filterOutcome) {
        return false;
      }
      return true;
    });
  }, [telemetryList, filterExecutor, filterOutcome]);

  // Unique executors for filter
  const executorOptions = useMemo(() => {
    const set = new Set<string>();
    for (const item of telemetryList) {
      set.add(item.executor);
    }
    return [
      { value: 'all', label: 'Todos los agentes' },
      ...Array.from(set).map((e) => ({ value: e, label: e })),
    ];
  }, [telemetryList]);

  const outcomeOptions = [
    { value: 'all', label: 'Todos los resultados' },
    { value: 'success', label: 'Éxito (success)' },
    { value: 'failure', label: 'Fallo (failure)' },
    { value: 'partial', label: 'Parcial (partial)' },
    { value: 'unknown', label: 'Desconocido (unknown)' },
  ];

  const handleStartEdit = (item: ExecutionTelemetry) => {
    setEditingId(item.id);
    setEditOutcome(item.real_outcome);
    setEditNote(item.outcome_note ?? '');
  };

  const handleSaveEdit = async (item: ExecutionTelemetry) => {
    if (!item.execution_process_id) return;
    setIsSaving(true);
    try {
      const payload: RecordExecutionTelemetry = {
        execution_process_id: item.execution_process_id,
        executor: item.executor,
        model_id: item.model_id,
        task_type: item.task_type,
        real_outcome: editOutcome,
        outcome_note: editNote.trim() ? editNote : null,
        cost_usd: item.cost_usd,
        duration_ms: item.duration_ms,
        input_tokens: item.input_tokens,
        output_tokens: item.output_tokens,
        total_tokens: item.total_tokens,
        recorded_by: 'manual',
      };
      await telemetryApi.record(payload);
      setEditingId(null);
      await loadData();
    } catch (err) {
      console.error('Failed to update telemetry verdict:', err);
    } finally {
      setIsSaving(false);
    }
  };

  const getOutcomeBadge = (outcome: string) => {
    switch (outcome) {
      case 'success':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-emerald-500/15 text-emerald-400 border border-emerald-500/30">
            <CheckCircleIcon className="w-3.5 h-3.5" />
            Éxito
          </span>
        );
      case 'failure':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-rose-500/15 text-rose-400 border border-rose-500/30">
            <XCircleIcon className="w-3.5 h-3.5" />
            Fallo
          </span>
        );
      case 'partial':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-amber-500/15 text-amber-400 border border-amber-500/30">
            <WarningCircleIcon className="w-3.5 h-3.5" />
            Parcial
          </span>
        );
      default:
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-zinc-500/15 text-zinc-400 border border-zinc-500/30">
            {outcome}
          </span>
        );
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-high">
            Telemetría de Agentes y Control de Costes
          </h2>
          <p className="text-xs text-low">
            Métricas de ejecución, consumo de tokens, coste real en USD y evaluación
            de resultados por modelo (Fase 4).
          </p>
        </div>
        <IconButton
          icon={isLoading ? SpinnerIcon : ArrowClockwiseIcon}
          onClick={() => void loadData()}
          title="Actualizar métricas"
          aria-label="Actualizar métricas"
          className={cn(isLoading && 'animate-spin')}
        />
      </div>

      {/* KPI Cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <div className="p-3.5 rounded-lg border border-border bg-panel flex flex-col gap-1">
          <div className="flex items-center gap-1.5 text-xs text-low font-medium">
            <CurrencyDollarIcon className="w-4 h-4 text-emerald-400" />
            Coste Total Acumulado
          </div>
          <div className="text-xl font-bold text-high font-mono">
            ${kpis.totalCost}
          </div>
          <div className="text-[10px] text-low">Facturación directa en USD</div>
        </div>

        <div className="p-3.5 rounded-lg border border-border bg-panel flex flex-col gap-1">
          <div className="flex items-center gap-1.5 text-xs text-low font-medium">
            <CpuIcon className="w-4 h-4 text-brand" />
            Turnos Ejecutados
          </div>
          <div className="text-xl font-bold text-high font-mono">
            {kpis.totalRuns}
          </div>
          <div className="text-[10px] text-low">Procesos completados</div>
        </div>

        <div className="p-3.5 rounded-lg border border-border bg-panel flex flex-col gap-1">
          <div className="flex items-center gap-1.5 text-xs text-low font-medium">
            <TimerIcon className="w-4 h-4 text-amber-400" />
            Duración Media
          </div>
          <div className="text-xl font-bold text-high font-mono">
            {kpis.avgDurationSec}s
          </div>
          <div className="text-[10px] text-low">Tiempo real por turno</div>
        </div>

        <div className="p-3.5 rounded-lg border border-border bg-panel flex flex-col gap-1">
          <div className="flex items-center gap-1.5 text-xs text-low font-medium">
            <CpuIcon className="w-4 h-4 text-purple-400" />
            Tokens Consumidos
          </div>
          <div className="text-xl font-bold text-high font-mono">
            {kpis.totalTokens}
          </div>
          <div className="text-[10px] text-low">Input + Output combinados</div>
        </div>
      </div>

      {/* Summary by Model */}
      <SettingsCard title="Rendimiento Comparativo por Modelo y Agente">
        {summaries.length === 0 ? (
          <div className="py-6 text-center text-xs text-low">
            No hay turnos registrados aún. Cuando los agentes ejecuten tareas, aparecerán las estadísticas aquí.
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-left text-xs">
              <thead>
                <tr className="border-b border-border text-low font-medium">
                  <th className="py-2 px-2">Agente</th>
                  <th className="py-2 px-2">Modelo</th>
                  <th className="py-2 px-2">Resultado</th>
                  <th className="py-2 px-2 text-right">Turnos</th>
                  <th className="py-2 px-2 text-right">Duración Media</th>
                  <th className="py-2 px-2 text-right">Coste Total</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border/50">
                {summaries.map((row, idx) => (
                  <tr key={idx} className="hover:bg-panel-hover/50">
                    <td className="py-2 px-2 font-medium text-high">
                      {row.executor}
                    </td>
                    <td className="py-2 px-2 text-normal font-mono">
                      {row.model_id ?? 'default'}
                    </td>
                    <td className="py-2 px-2">
                      {getOutcomeBadge(row.real_outcome)}
                    </td>
                    <td className="py-2 px-2 text-right font-mono text-high">
                      {Number(row.count)}
                    </td>
                    <td className="py-2 px-2 text-right font-mono text-low">
                      {row.avg_duration_ms != null
                        ? `${(row.avg_duration_ms / 1000).toFixed(1)}s`
                        : '—'}
                    </td>
                    <td className="py-2 px-2 text-right font-mono text-emerald-400 font-semibold">
                      {row.total_cost_usd != null
                        ? `$${row.total_cost_usd.toFixed(4)}`
                        : '—'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </SettingsCard>

      {/* Detailed Telemetry History */}
      <SettingsCard title="Historial Detallado de Telemetría y Evaluación Humana (HITL)">
        <div className="flex flex-wrap items-center gap-3 mb-4">
          <div className="w-48">
            <SettingsSelect
              value={filterExecutor}
              options={executorOptions}
              onChange={(val) => setFilterExecutor(val)}
            />
          </div>
          <div className="w-48">
            <SettingsSelect
              value={filterOutcome}
              options={outcomeOptions}
              onChange={(val) => setFilterOutcome(val)}
            />
          </div>
          <div className="text-xs text-low ml-auto">
            Mostrando {filteredList.length} de {telemetryList.length} registros
          </div>
        </div>

        {filteredList.length === 0 ? (
          <div className="py-8 text-center text-xs text-low">
            No se encontraron registros de telemetría con los filtros seleccionados.
          </div>
        ) : (
          <div className="space-y-2.5">
            {filteredList.map((item) => {
              const isEditing = editingId === item.id;
              const dateStr = new Date(item.created_at).toLocaleString();

              return (
                <div
                  key={item.id}
                  className="p-3 rounded-md border border-border bg-panel/60 hover:bg-panel transition-colors text-xs space-y-2"
                >
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <div className="flex items-center gap-2">
                      <span className="font-semibold text-high">
                        {item.executor}
                      </span>
                      {item.model_id && (
                        <span className="font-mono text-low bg-border/40 px-1.5 py-0.5 rounded text-[11px]">
                          {item.model_id}
                        </span>
                      )}
                      <span className="text-[10px] text-low uppercase tracking-wider">
                        [{item.task_type}]
                      </span>
                      <span className="text-[10px] text-low/80">
                        vía {item.recorded_by}
                      </span>
                    </div>

                    <div className="flex items-center gap-3">
                      {getOutcomeBadge(item.real_outcome)}
                      <span className="text-[11px] text-low font-mono">
                        {dateStr}
                      </span>
                    </div>
                  </div>

                  {/* Metrics Row */}
                  <div className="flex flex-wrap items-center gap-4 text-low text-[11px] font-mono bg-border/20 p-2 rounded">
                    <div>
                      Coste:{' '}
                      <span className="text-emerald-400 font-semibold">
                        {item.cost_usd != null
                          ? `$${item.cost_usd.toFixed(4)}`
                          : '—'}
                      </span>
                    </div>
                    <div>
                      Duración:{' '}
                      <span className="text-high">
                        {item.duration_ms != null
                          ? `${(Number(item.duration_ms) / 1000).toFixed(1)}s`
                          : '—'}
                      </span>
                    </div>
                    <div>
                      Tokens In:{' '}
                      <span className="text-high">
                        {item.input_tokens != null
                          ? Number(item.input_tokens).toLocaleString()
                          : '—'}
                      </span>
                    </div>
                    <div>
                      Tokens Out:{' '}
                      <span className="text-high">
                        {item.output_tokens != null
                          ? Number(item.output_tokens).toLocaleString()
                          : '—'}
                      </span>
                    </div>
                    <div>
                      Tokens Total:{' '}
                      <span className="text-high font-bold">
                        {item.total_tokens != null
                          ? Number(item.total_tokens).toLocaleString()
                          : '—'}
                      </span>
                    </div>
                  </div>

                  {/* Note & Action */}
                  <div className="flex items-center justify-between gap-2 pt-1">
                    <div className="text-low truncate max-w-xl">
                      {item.outcome_note ? (
                        <span>Nota: {item.outcome_note}</span>
                      ) : (
                        <span className="italic text-low/60">Sin notas de auditoría</span>
                      )}
                    </div>

                    {item.execution_process_id && !isEditing && (
                      <button
                        type="button"
                        onClick={() => handleStartEdit(item)}
                        className="text-[11px] text-brand hover:underline font-medium"
                      >
                        Auditar veredicto
                      </button>
                    )}
                  </div>

                  {/* Inline Edit Form */}
                  {isEditing && (
                    <div className="pt-2 border-t border-border mt-2 space-y-2 bg-panel p-2.5 rounded">
                      <div className="text-xs font-semibold text-high">
                        Auditoría de Veredicto Humano (HITL)
                      </div>
                      <div className="flex gap-2">
                        <div className="w-48">
                          <SettingsSelect
                            value={editOutcome}
                            options={[
                              { value: 'success', label: 'Éxito (success)' },
                              { value: 'failure', label: 'Fallo (failure)' },
                              { value: 'partial', label: 'Parcial (partial)' },
                              { value: 'unknown', label: 'Desconocido (unknown)' },
                            ]}
                            onChange={(v) => setEditOutcome(v)}
                          />
                        </div>
                        <input
                          type="text"
                          value={editNote}
                          onChange={(e) => setEditNote(e.target.value)}
                          placeholder="Nota o motivo de corrección..."
                          className="flex-1 bg-input border border-border rounded px-2.5 py-1 text-xs text-high focus:outline-none focus:border-brand"
                        />
                      </div>
                      <div className="flex justify-end gap-2">
                        <button
                          type="button"
                          onClick={() => setEditingId(null)}
                          className="px-2.5 py-1 rounded text-xs text-low hover:bg-panel-hover"
                        >
                          Cancelar
                        </button>
                        <PrimaryButton
                          disabled={isSaving}
                          onClick={() => void handleSaveEdit(item)}
                          className="text-xs py-1 px-3"
                        >
                          {isSaving ? 'Guardando...' : 'Guardar veredicto'}
                        </PrimaryButton>
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </SettingsCard>
    </div>
  );
}
