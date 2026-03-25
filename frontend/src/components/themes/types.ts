export type UITheme = "default" | "warcraft" | "cyberpunk" | "forest" | "ocean" | "pokemon" | "chucknorris" | "lotr";

export interface ThemeText {
  // Branding
  appName: string;
  tagline: string;

  // Navigation
  dashboard: string;
  executions: string;
  workflowDefs: string;
  taskDefs: string;
  taskQueues: string;

  // Actions
  startWorkflow: string;
  startingWorkflow: string;
  search: string;
  newDefinition: string;
  newTaskDef: string;
  viewAll: string;
  cancel: string;
  delete: string;
  edit: string;

  // Dashboard
  dashboardTitle: string;
  dashboardSubtitle: string;
  running: string;
  completed: string;
  failed: string;
  queuedTasks: string;
  healthy: string;
  offline: string;
  statusDistribution: string;
  workflowDefinitions: string;
  taskDefinitions: string;
  registeredDefinitions: string;
  registeredTaskTypes: string;
  recentExecutions: string;
  noRecentExecutions: string;

  // Executions page
  executionsTitle: string;
  autoRefresh: string;
  live: string;
  searchAndFilter: string;
  searchPlaceholder: string;
  allWorkflowTypes: string;
  results: string;
  showing: string;
  workflow: string;
  status: string;
  started: string;
  ended: string;
  actions: string;
  prev: string;
  next: string;
  page: string;
  of: string;
  pause: string;
  terminate: string;
  resume: string;
  restart: string;
  terminateWorkflow: string;
  terminateConfirm: string;

  // Workflow Defs page
  workflowDefsTitle: string;
  filterDefinitions: string;
  noWorkflowDefs: string;
  noDefsMatch: string;
  name: string;
  version: string;
  tasks: string;
  description: string;
  timeout: string;
  startExecution: string;
  viewJson: string;
  cloneAndEdit: string;
  createWorkflowDef: string;
  editWorkflowDef: string;
  deleteWorkflowDef: string;

  // Task Defs page
  taskDefsTitle: string;
  filterTaskDefs: string;
  noTaskDefs: string;
  retryCount: string;
  responseTimeout: string;
  owner: string;
  testRun: string;
  view: string;
  createTaskDef: string;
  deleteTaskDef: string;
  taskInput: string;
  runTest: string;

  // Task Queues page
  taskQueuesTitle: string;
  totalQueued: string;
  filterQueues: string;
  activeQueues: string;
  noQueuesMatch: string;
  noTasksInQueue: string;

  // Command palette
  searchAll: string;
  pages: string;
  noResults: string;

  // Start workflow dialog
  startWorkflowTitle: string;
  startWorkflowDesc: string;
  workflowLabel: string;
  versionLabel: string;
  inputLabel: string;
  correlationId: string;
  selectWorkflow: string;
  loadingWorkflows: string;
  noWorkflowsFound: string;

  // Breadcrumbs
  breadcrumbDashboard: string;
  breadcrumbExecutions: string;
  breadcrumbDefinitions: string;
  breadcrumbTaskDefs: string;
  breadcrumbQueues: string;

  // Loading
  loading: string;

  // Toasts
  toastWorkflowTerminated: string;
  toastWorkflowPaused: string;
  toastWorkflowResumed: string;
  toastWorkflowRestarted: string;
  toastWorkflowRetried: string;
  toastWorkflowStarted: string;
  toastWorkflowDefSaved: string;
  toastTaskDefCreated: string;
  toastDeleted: string;
  toastInvalidJson: string;

  // Workflow detail
  workflowNotFound: string;
  retry: string;
  tasksTab: string;
  timeline: string;
  diagram: string;
  input: string;
  output: string;
  info: string;
  task: string;
  type: string;
  worker: string;
  reason: string;
  viewSubWorkflow: string;
  loadingWorkflowDef: string;
  workflowIdLabel: string;
  correlationIdLabel: string;
  priority: string;
  updated: string;
  scheduled: string;
  pollCount: string;
  duration: string;

  // Confirm dialog
  confirm: string;
  pleaseWait: string;

  // Status filters
  allStatuses: string;

  // Workflow builder
  createWorkflow: string;
  saveWorkflow: string;
  visualBuilder: string;
  jsonEditor: string;
  saving: string;
  addTask: string;
  noTasksYet: string;
  tasksSequentialHint: string;
  workflowNameRequired: string;
  addAtLeastOneTask: string;
  allTasksNeedRefName: string;
  previewJson: string;

  // Workflow builder settings
  workflowSettings: string;
  nameRequired: string;
  descriptionPlaceholder: string;
  timeoutSeconds: string;
  ownerEmail: string;
  failureWorkflow: string;
  optional: string;
  inputParameters: string;
  inputParamPlaceholder: string;
  inputParamHint: string;

  // Task card
  unnamed: string;
  moveUp: string;
  moveDown: string;
  duplicate: string;
  remove: string;
  taskType: string;
  referenceNameRequired: string;
  taskName: string;
  selectRegisteredTask: string;
  startDelay: string;

  // Task def form
  retryLogic: string;
  retryFixed: string;
  retryExponential: string;
  retryLinear: string;
  retryDelay: string;
  timeoutPolicy: string;
  timeoutPolicyTimeOut: string;
  timeoutPolicyAlert: string;
  timeoutPolicyRetry: string;
  responseTimeoutLabel: string;
  concurrentExecLimit: string;
  inputKeys: string;
  outputKeys: string;
  creating: string;
  createFromJson: string;
  form: string;
  json: string;

  // Task type fields
  subWorkflowConfig: string;
  workflowName: string;
  versionOptional: string;
  parallelBranches: string;
  addBranch: string;
  forkJoinNote: string;
  decisionConfig: string;
  caseExpression: string;
  caseValueParam: string;
  cases: string;
  addCase: string;
  doWhileConfig: string;
  loopCondition: string;
  loopBodyTasks: string;

  // Task input fields
  source: string;
  customValue: string;
  selectParameter: string;
  noInputKeys: string;
  selectTaskForInputs: string;
  provideParamsJson: string;
  pickSourceHint: string;

  // Misc
  testRunDescription: string;
  autoParseHint: string;
  deleteConfirmSuffix: string;
  liveInterval: string;
  copyToClipboard: string;
  none: string;

  // Relative time
  justNow: string;
  secondsAgo: string;
  minutesAgo: string;
  hoursAgo: string;
  daysAgo: string;

  // Layout & chrome
  expandSidebar: string;
  collapseSidebar: string;
  commandPalette: string;
  themeLabel: string;
  uiTheme: string;
  autoRefreshOn: string;
  autoRefreshOff: string;

  // Toggle labels
  fields: string;

  // Placeholders & hints
  valueFor: string;
  typeAndPressEnter: string;
  typeKeyAndEnter: string;
  taskDescriptionHint: string;
  noTimeoutHint: string;
  unlimitedHint: string;
  latest: string;
  referenceName: string;
  caseValue: string;

  // Diagram node labels
  diagramStart: string;
  diagramEnd: string;
  diagramFork: string;
  diagramJoin: string;
  diagramSwitch: string;
  diagramSubWf: string;
  diagramLoop: string;
  diagramEvent: string;
  diagramHttp: string;
  diagramWait: string;
  diagramTask: string;

  // Test workflow
  testWorkflowDesc: string;

  // About page
  about: string;
  aboutTitle: string;
  aboutSubtitle: string;
  aboutFooter: string;
  breadcrumbAbout: string;

  // Version history (#71)
  versionHistory: string;
  compareVersions: string;
  noOtherVersions: string;
  noDifferences: string;
  added: string;
  removed: string;

  // Dashboard customization (#72)
  customizeDashboard: string;
  resetLayout: string;
  widgetVisible: string;

  // Queue alerting (#73)
  alertThreshold: string;
  thresholdExceeded: string;
  configureAlerts: string;
  clearThreshold: string;

  // Search all versions (#74)
  searchAllVersions: string;

  // Execution replay (#75)
  replayExecution: string;
  replayDesc: string;

  // Dependency graph (#76)
  dependencyGraph: string;
  dependencyGraphDesc: string;
  noDependencies: string;
  breadcrumbDependencies: string;

  // Schedules
  schedules: string;
  schedulesTitle: string;
  breadcrumbSchedules: string;
  createSchedule: string;
  editSchedule: string;
  deleteSchedule: string;
  cronExpression: string;
  timezoneLabel: string;
  enabled: string;
  disabled: string;
  lastRun: string;
  nextRun: string;
  noSchedules: string;
  scheduleCreated: string;
  scheduleDeleted: string;
  scheduleEnabled: string;
  scheduleDisabled: string;
  filterSchedules: string;
  scheduleLastError: string;

  // Webhooks
  onCompleteWebhook: string;
  onFailureWebhook: string;
  webhookPlaceholder: string;

  // SLA
  slaDeadline: string;
  slaHint: string;

  // Tags
  tagsLabel: string;
  tagsPlaceholder: string;

  // Bulk operations
  bulkActions: string;
  bulkPause: string;
  bulkResume: string;
  bulkRetry: string;
  bulkRestart: string;
  bulkTerminate: string;
  bulkConfirm: string;
  selectedCount: string;
  selectAll: string;
  deselectAll: string;
  bulkSuccess: string;

  // Task def extras
  retryOnErrors: string;
  retryOnErrorsHint: string;
  envVars: string;
  envVarsHint: string;
}
