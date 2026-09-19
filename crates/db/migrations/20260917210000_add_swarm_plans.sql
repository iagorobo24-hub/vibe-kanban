-- Fase 7 (orquestador y swarm, docs/agentos/04-ROADMAP.md).
-- Revisión de ADR-017: los planes de swarm dejan de vivir en un HashMap en
-- memoria y pasan a SQLite. La condición que la propia ADR fijó para revisarse
-- ("cuando el swarm empiece a ejecutar subtareas reales contra el runtime") se
-- cumplió: a partir de aquí un plan huérfano tras un reinicio ya no es
-- inofensivo, porque puede haber un proceso real corriendo sin plan que lo
-- explique.
--
-- El plan se guarda como documento JSON completo en `plan_json`. Es deliberado:
-- las subtareas todavía no son entidades relacionales (no se consultan por sí
-- solas ni se cruzan con telemetría a nivel SQL). `goal_id` y `status` sí se
-- sacan a columnas porque se filtran y se ordenan. Migrar las subtareas a su
-- propia tabla es deuda abierta, no un olvido.
PRAGMA foreign_keys = ON;

CREATE TABLE swarm_plans (
    goal_id     BLOB PRIMARY KEY,
    plan_json   TEXT NOT NULL,
    status      TEXT NOT NULL
                  CHECK (status IN ('DRAFT','APPROVED','EXECUTING','COMPLETED','FAILED')),
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX idx_swarm_plans_status ON swarm_plans(status);
CREATE INDEX idx_swarm_plans_created_at ON swarm_plans(created_at);
