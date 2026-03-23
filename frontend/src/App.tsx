import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter, Routes, Route, Navigate } from "react-router";
import { Toaster } from "sonner";
import { ThemeProvider } from "./components/ThemeContext";
import Layout from "./components/Layout";
import Dashboard from "./pages/Dashboard";
import Workflows from "./pages/Workflows";
import WorkflowDetail from "./pages/WorkflowDetail";
import WorkflowDefs from "./pages/WorkflowDefs";
import TaskDefs from "./pages/TaskDefs";
import TaskQueues from "./pages/TaskQueues";
import WorkflowDependencyGraph from "./pages/WorkflowDependencyGraph";
import Schedules from "./pages/Schedules";
import About from "./pages/About";

const queryClient = new QueryClient({
  defaultOptions: { queries: { refetchOnWindowFocus: false, retry: 1 } },
});

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <BrowserRouter>
          <Routes>
            <Route element={<Layout />}>
              <Route path="/" element={<Dashboard />} />
              <Route path="/executions" element={<Workflows />} />
              <Route path="/executions/:id" element={<WorkflowDetail />} />
              <Route path="/definitions" element={<WorkflowDefs />} />
              <Route path="/taskdefs" element={<TaskDefs />} />
              <Route path="/queues" element={<TaskQueues />} />
              <Route path="/dependencies" element={<WorkflowDependencyGraph />} />
              <Route path="/schedules" element={<Schedules />} />
              <Route path="/about" element={<About />} />
              <Route path="*" element={<Navigate to="/" replace />} />
            </Route>
          </Routes>
        </BrowserRouter>
        <Toaster richColors position="top-right" />
      </ThemeProvider>
    </QueryClientProvider>
  );
}
