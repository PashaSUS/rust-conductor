import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter, Routes, Route, Navigate } from "react-router";
import { Toaster } from "sonner";
import Layout from "./components/Layout";
import Dashboard from "./pages/Dashboard";
import Workflows from "./pages/Workflows";
import WorkflowDetail from "./pages/WorkflowDetail";
import WorkflowDefs from "./pages/WorkflowDefs";
import TaskDefs from "./pages/TaskDefs";
import TaskQueues from "./pages/TaskQueues";

const queryClient = new QueryClient({
  defaultOptions: { queries: { refetchOnWindowFocus: false, retry: 1 } },
});

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Routes>
          <Route element={<Layout />}>
            <Route path="/" element={<Dashboard />} />
            <Route path="/executions" element={<Workflows />} />
            <Route path="/executions/:id" element={<WorkflowDetail />} />
            <Route path="/definitions" element={<WorkflowDefs />} />
            <Route path="/taskdefs" element={<TaskDefs />} />
            <Route path="/queues" element={<TaskQueues />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      </BrowserRouter>
      <Toaster richColors position="top-right" />
    </QueryClientProvider>
  );
}
