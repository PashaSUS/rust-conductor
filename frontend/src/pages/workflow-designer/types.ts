import type { ReactNode } from "react";
import type { TaskInputParameterDef } from "@/api/conductor-types";

export interface ContextMenuItem {
  label: string;
  icon?: ReactNode;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
}

export interface DesignerTask {
  id: string;
  name: string;
  taskReferenceName: string;
  type: string;
  description: string;
  inputParameters: Record<string, unknown>;
  /** Optional rich descriptions for each entry of `inputParameters`. */
  inputParameterDefinitions?: TaskInputParameterDef[];
  outputKeys: string[];
  subWorkflowParam?: { name: string; version?: number };
  optional?: boolean;
  // Decision/Switch branching
  caseExpression?: string;
  caseValueParam?: string;
  caseNames?: string[];
  // Do While loop
  loopCondition?: string;
}
