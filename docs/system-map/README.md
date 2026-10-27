---
title: "Project System Map — Discovery draft"
description: "Unverified repository architecture discovery snapshot."
---

# System Map — vibe-kanban

**Estado:** `draft` · **Perfil provisional:** `LITE` · **Fecha:** 2026-10-27  
**Referencia GitHub examinada:** `main` @ `4deb7eca8f381f7cbc1f9d15515a9ab8f8009053`  
**Evidencia:** existencia de documentos, no comportamiento ni contratos arquitectónicos.

## Catálogo de enrutamiento

| Documento | Ruta | Estado |
|---|---|---|
| `README.md` | [Abrir](../../README.md) | Ruta existente, relevancia arquitectónica pendiente de revisar |
| `AGENTS.md` | [Abrir](../../AGENTS.md) | Ruta existente, relevancia arquitectónica pendiente de revisar |
| `CLAUDE.md` | [Abrir](../../CLAUDE.md) | Ruta existente, relevancia arquitectónica pendiente de revisar |
| `docs/AGENTS.md` | [Abrir](../../docs/AGENTS.md) | Ruta existente, relevancia arquitectónica pendiente de revisar |
| `docs/system-map/manifest.yaml` | [Metadatos](./manifest.yaml) | Snapshot, perfil provisional y cobertura desconocida |

## Why this shape

Este borrador indexa únicamente archivos vistos en GitHub. Evita duplicar el contenido canónico del repositorio o inventar componentes, dependencias y flujos. `psm.source.commit` señala el punto de descubrimiento remoto, NO certifica una auditoría de arquitectura. `LITE` es provisional, y debe cambiarse a `FULL` si el discovery revela múltiples dominios materiales.

## Impacto e invariantes

- `Domains: UNKNOWN`; `flows: UNKNOWN`; `boundaries: UNKNOWN`; `invariants: UNKNOWN`.
- **Hits / Does not hit:** todavía no determinados; no hacer cambios materiales sin delimitar el contrato de impacto contra código y docs canónicas.
- Ninguna prueba runtime, seguridad, despliegue o integridad semántica fue ejecutada.

## Pendientes de activación

1. Leer reglas de contribución, arquitectura y código relevante; reconciliar la fuente canónica con la implementación.
2. Redactar dominios/flows/boundaries/invariantes con evidencia `VERIFIED|REPORTED|INFERRED|UNKNOWN` y rutas reales, y verificar excepciones.
3. Cerrar `MECHANICAL_GATE`, `SEMANTIC_AGENT_REVIEW` y Cold-Agent Walk antes de cambiar `status` a `active`.

**Validación:** `MECHANICAL_GATE: UNAVAILABLE`; `SEMANTIC_AGENT_REVIEW: PENDING`; `RUNTIME: NOT RUN`. No se ha consultado el árbol local.
