# Frontend — React Dashboard

rust-conductor includes a **single-page application (SPA)** dashboard for managing workflows, task definitions, schedules, and monitoring execution state.

## Why a Built-In Frontend?

| Alternative | Why Not |
|-------------|---------|
| CLI only | Workflows have complex state (DAGs, nested tasks, timelines) — hard to visualize in a terminal |
| Third-party UI (Conductor UI) | Netflix Conductor UI is Java/Node-based and tightly coupled to their API format |
| No UI (API only) | Operators need to debug stuck workflows, inspect task I/O, and trigger manual actions without scripting |

A built-in frontend gives operators immediate visibility into the system without additional deployment or integration.

---

## Tech Stack

### Why React 19?

| Requirement | Why React |
|-------------|-----------|
| Component reuse | Dashboard has many repeated patterns (status badges, tables, dialogs) — components compose well |
| Ecosystem | shadcn/ui, Radix, React Query, XYFlow — all React-native libraries |
| Developer pool | Most widely known frontend framework |

### Why Vite + SWC?

| Alternative | Why Vite Wins |
|-------------|---------------|
| Webpack | Slower dev server startup, more complex config |
| Create React App | Deprecated, no longer maintained |
| Next.js | SSR/SSG unnecessary for an admin dashboard |
| Parcel | Smaller ecosystem, fewer plugins |

Vite provides sub-second dev server startup via native ES modules, and SWC transpiles TypeScript ~20× faster than Babel.

### Why TypeScript?

The API client interfaces (`WorkflowDef`, `TaskResult`, `SearchParams`) mirror the backend models. TypeScript catches mismatches at compile time — e.g., accessing `wf.workflowId` when the field was renamed to `wf.workflow_id`.

### Why TanStack React Query?

| Alternative | Why React Query Wins |
|-------------|---------------------|
| `useEffect` + `useState` | No caching, no background refetch, no deduplication, manual loading/error state |
| Redux + RTK Query | Heavier abstraction, more boilerplate, separate store to manage |
| SWR | Similar but fewer features (no mutation management, weaker devtools) |
| Apollo Client | Designed for GraphQL, not REST |

React Query provides:
- **Stale-while-revalidate** caching — UI shows cached data instantly, fetches fresh data in background
- **Automatic refetching** — running workflows auto-refresh every 3-5 seconds
- **Mutation invalidation** — pausing a workflow automatically refreshes the workflow list
- **Request deduplication** — multiple components requesting the same data share one network call

### Why Tailwind CSS 4?

| Alternative | Why Tailwind Wins |
|-------------|-------------------|
| CSS Modules | Verbose, scattered files, no design system |
| Styled Components | Runtime CSS-in-JS overhead, bundle size |
| Plain CSS | No constraints, inconsistent spacing/colors across developers |

Tailwind provides a constraint-based design system (consistent spacing, colors, typography) with zero-runtime CSS output. Version 4 adds native CSS nesting and the Vite plugin for faster builds.

### Why shadcn/ui + Radix UI?

| Alternative | Why shadcn/ui Wins |
|-------------|-------------------|
| Material UI | Opinionated design, hard to customize, large bundle |
| Ant Design | Same issues, plus CJK-first documentation |
| Headless UI | Fewer components, Tailwind Labs only |
| Custom components | Accessibility is hard — WAI-ARIA patterns for dialogs, selects, and tooltips are complex |

shadcn/ui provides **copy-paste components** built on Radix UI accessibility primitives. No external dependency — components live in the project. Fully customizable via Tailwind without CSS overrides.

### Why @xyflow/react?

Workflow definitions are directed acyclic graphs (DAGs). Workers, forks, joins, and decisions form a visual tree. XYFlow renders interactive, zoomable, pannable diagrams with:
- Node positioning (dagre/ELK layout algorithms)
- Edge routing (bezier curves)
- Click handlers (click a task to see its execution details)

No other React library handles this as well. D3-based alternatives require much more manual work for interactive node graphs.

---

## Architecture

### Routing

```
/                     → Dashboard (stats, recent executions, health, customizable widgets)
/executions           → Workflow search (filter by status, name, free text, tags, bulk ops)
/executions/:id       → Workflow detail (tasks, timeline, flame chart, diagram, replay, I/O)
/definitions          → Workflow definitions (version history, clone, diagram preview)
/definitions/create   → Create/edit workflow definition (full-page editor)
/taskdefs             → Task definitions (CRUD, test run)
/taskdefs/create      → Create task definition (full-page editor)
/queues               → Task queue depths (alert thresholds, utilization gauges)
/schedules            → CRON schedule management (timezone support)
/metrics              → Workflow execution metrics (percentiles, success rates)
/dependencies         → Workflow dependency graph (root/leaf node identification)
/compare              → Execution comparison (side-by-side diff, timeline overlay) [lazy]
/diff                 → Workflow definition version diff (LCS-based visual diff) [lazy]
/designer             → Visual workflow designer (drag-and-drop, React Flow) [lazy]
/stresser             → Workflow stress tester (load testing, random data) [lazy]
/templates            → Workflow template marketplace (built-in patterns) [lazy]
/about                → System info & tech stack
```

Routes marked `[lazy]` are code-split via `React.lazy()` with a Suspense fallback.

### API Client

**File:** `src/api/conductor.ts`

A thin typed wrapper over `fetch()`:

```typescript
async function request<T>(path: string, options?: RequestInit): Promise<T>
```

**Why no code generation (OpenAPI / proto)?** The API surface is stable and small enough that manually typed interfaces are simpler to maintain. Code generation adds build-time tooling, generated file management, and version sync headaches.

**Dev proxy:** Vite proxies `/api` and `/health` to `http://localhost:8090`, avoiding CORS configuration during development.

### State Management

**Pattern:** Server state via React Query, local UI state via `useState` / `useRef`.

**No client-side store** (Redux, Zustand, Jotai). React Query manages all server data — caching, background refetching, mutations, and cache invalidation. This eliminates the "stale UI" problem where local state diverges from server state.

### Shared Hooks

| Hook | Purpose |
|------|---------|
| `useWorkflowMutations(invalidateKeys)` | Reusable mutations: pause, resume, terminate, restart, retry |
| `useBulkWorkflowMutations(invalidateKeys, onSuccess)` | Bulk variants of the above |

**Why shared hooks?** Workflow actions (pause, resume, etc.) appear on multiple pages (Workflows list, WorkflowDetail, Dashboard). Without shared hooks, each page had ~50 lines of identical mutation boilerplate.

### Shared Utilities

| Module | Purpose |
|--------|---------|
| `lib/field-parser.ts` | Parse string field values to typed values (number, boolean, JSON, null) |
| `components/StatusBadge.tsx` | Color-coded status badges for workflows and tasks |

---

## Theme System

**File:** `src/components/ThemeContext.tsx`

The theme system combines visual theming with text localization. Each theme provides:
- A CSS class applied to the document root (color overrides via Tailwind custom properties)
- A complete set of UI strings (labels, button text, toast messages)

**Available themes:** default, warcraft, cyberpunk, forest, ocean, pokemon, chucknorris, lotr

**Why themes with localized text?** Started as a fun feature, but demonstrates the i18n architecture. Each theme's text object is a drop-in replacement — switching themes is equivalent to switching languages.

**Storage:** `localStorage` — no server round-trip, instant on page load.

---

## Build & Deployment

### Development

```bash
cd frontend
npm install
npm run dev     # Vite dev server on :3170, proxies /api to :8090
```

### Production

```bash
npm run build   # Output to dist/
```

The `Dockerfile` uses a multi-stage build:
1. `node:24` — install dependencies, run Vite build
2. `nginx:alpine` — serve static assets, proxy `/api` to the backend

### Nginx Configuration

**File:** `frontend/nginx.conf`

```nginx
location /api { proxy_pass http://backend:8090; }
location /health { proxy_pass http://backend:8090; }
location / { try_files $uri /index.html; }   # SPA fallback
```

---

## Key Dependencies

| Package | Version | Why This Library |
|---------|---------|-----------------|
| `react` | 19 | UI framework — component model, hooks, concurrent features |
| `react-router` | 7 | Client-side routing — nested routes, data loading |
| `@tanstack/react-query` | 5 | Server state — caching, refetch, mutations, devtools |
| `@tanstack/react-table` | 8 | Headless table — sorting, filtering, pagination without UI lock-in |
| `@xyflow/react` | — | Workflow DAG diagram rendering |
| `recharts` | — | Dashboard charts (status distribution, execution counts) |
| `@radix-ui/*` | — | Accessible UI primitives (dialog, dropdown, select, tabs, tooltip) |
| `tailwindcss` | 4 | Utility-first CSS — consistent design system, zero runtime |
| `lucide-react` | — | Icon library — consistent, tree-shakeable SVG icons |
| `sonner` | — | Toast notifications — minimal, animated, accessible |
| `vite` | 7 | Build tool — fast dev server, optimized production builds |
| `typescript` | ~5.9 | Type safety — catch API mismatches at compile time |

---

## Code Reference

| File | Role |
|------|------|
| `src/App.tsx` | Route definitions, providers (QueryClient, Theme, Router) |
| `src/api/conductor.ts` | Typed API client — all backend communication |
| `src/components/Layout.tsx` | Sidebar + navbar shell, keyboard shortcuts |
| `src/components/ThemeContext.tsx` | Theme provider, localized text |
| `src/components/StatusBadge.tsx` | Color-coded status badges for workflows and tasks |
| `src/components/WorkflowDiagram.tsx` | XYFlow-based DAG rendering |
| `src/components/WorkflowBuilder.tsx` | Visual workflow definition editor |
| `src/components/FlameChart.tsx` | Execution flame chart visualization |
| `src/components/TaskDependencyGraph.tsx` | Task dependency graph with critical path |
| `src/components/DataExplorer.tsx` | Nested JSON data explorer with breadcrumbs |
| `src/components/ReplayPlayer.tsx` | Execution replay animation |
| `src/components/NotificationBell.tsx` | Real-time failure/SLA alert notifications |
| `src/components/ErrorBoundary.tsx` | Error boundary with retry UI |
| `src/components/CommandPalette.tsx` | Keyboard-driven command palette |
| `src/components/StartWorkflowDialog.tsx` | Quick workflow start dialog |
| `src/hooks/useWorkflowMutations.ts` | Shared mutation hooks (pause, resume, terminate, etc.) |
| `src/hooks/useBulkWorkflowMutations.ts` | Bulk workflow operation hooks |
| `src/hooks/usePagination.ts` | Client-side pagination hook |
| `src/lib/field-parser.ts` | Input field parsing utilities |
| `src/pages/Dashboard.tsx` | Overview stats, health, customizable widgets |
| `src/pages/Workflows.tsx` | Workflow search + bulk actions |
| `src/pages/WorkflowDetail.tsx` | Execution detail (9 tabs: tasks, timeline, flame chart, etc.) |
| `src/pages/TaskDefs.tsx` | Task definition management |
| `src/pages/Schedules.tsx` | CRON schedule management |
| `src/pages/WorkflowMetrics.tsx` | Execution analytics (percentiles, rates) |
| `src/pages/ExecutionComparison.tsx` | Side-by-side execution diff |
| `src/pages/WorkflowDiff.tsx` | Definition version diff (LCS-based) |
| `src/pages/WorkflowDesigner.tsx` | Drag-and-drop visual workflow builder |
| `src/pages/WorkflowStresser.tsx` | Load testing tool |
| `src/pages/TemplateMarketplace.tsx` | Built-in workflow template library |
