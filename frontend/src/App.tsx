import { lazy, Suspense } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter, Routes, Route, Navigate } from "react-router";
import { Toaster } from "sonner";
import { ThemeProvider } from "./components/ThemeContext";
import { ErrorBoundary } from "./components/ErrorBoundary";
import Layout from "./components/Layout";
import Dashboard from "./pages/Dashboard";
import Workflows from "./pages/Workflows";
import WorkflowDetail from "./pages/workflow-detail";
import WorkflowDefs from "./pages/WorkflowDefs";
import TaskDefs from "./pages/TaskDefs";
import TaskQueues from "./pages/TaskQueues";
import WorkflowDependencyGraph from "./pages/WorkflowDependencyGraph";
import Schedules from "./pages/Schedules";
import WorkflowMetrics from "./pages/WorkflowMetrics";
import About from "./pages/About";
import CreateWorkflowDef from "./pages/CreateWorkflowDef";
import CreateTaskDef from "./pages/CreateTaskDef";

// Code-split heavy pages
const ExecutionComparison = lazy(() => import("./pages/ExecutionComparison"));
const WorkflowDiff = lazy(() => import("./pages/WorkflowDiff"));
const TemplateMarketplace = lazy(() => import("./pages/TemplateMarketplace"));
const WorkflowDesigner = lazy(() => import("./pages/workflow-designer"));
const WorkflowStresser = lazy(() => import("./pages/stresser"));
const WorkflowValidation = lazy(() => import("./pages/WorkflowValidation"));
const WorkflowSignals = lazy(() => import("./pages/WorkflowSignals"));

function LazyFallback() {
  return (
    <div className="flex items-center justify-center py-16">
      <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary" />
    </div>
  );
}

const queryClient = new QueryClient({
  defaultOptions: { queries: { refetchOnWindowFocus: false, retry: 1 } },
});

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <ErrorBoundary>
          <BrowserRouter>
            <Routes>
              <Route element={<Layout />}>
              <Route path="/" element={<Dashboard />} />
              <Route path="/executions" element={<Workflows />} />
              <Route path="/executions/:id" element={<WorkflowDetail />} />
              <Route path="/definitions" element={<WorkflowDefs />} />
              <Route path="/definitions/create" element={<CreateWorkflowDef />} />
              <Route path="/taskdefs" element={<TaskDefs />} />
              <Route path="/taskdefs/create" element={<CreateTaskDef />} />
              <Route path="/queues" element={<TaskQueues />} />
              <Route path="/dependencies" element={<WorkflowDependencyGraph />} />
              <Route path="/compare" element={<Suspense fallback={<LazyFallback />}><ExecutionComparison /></Suspense>} />
              <Route path="/diff" element={<Suspense fallback={<LazyFallback />}><WorkflowDiff /></Suspense>} />
              <Route path="/templates" element={<Suspense fallback={<LazyFallback />}><TemplateMarketplace /></Suspense>} />
              <Route path="/designer" element={<Suspense fallback={<LazyFallback />}><WorkflowDesigner /></Suspense>} />
              <Route path="/stresser" element={<Suspense fallback={<LazyFallback />}><WorkflowStresser /></Suspense>} />
              <Route path="/validate" element={<Suspense fallback={<LazyFallback />}><WorkflowValidation /></Suspense>} />
              <Route path="/signals" element={<Suspense fallback={<LazyFallback />}><WorkflowSignals /></Suspense>} />
              <Route path="/schedules" element={<Schedules />} />
              <Route path="/metrics" element={<WorkflowMetrics />} />
              <Route path="/about" element={<About />} />
              <Route path="*" element={<Navigate to="/" replace />} />
            </Route>
          </Routes>
        </BrowserRouter>
        </ErrorBoundary>
        <Toaster richColors position="top-right" />
      </ThemeProvider>
    </QueryClientProvider>
  );
}
