import { useThemeText } from "@/components/ThemeContext";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Server,
  Database,
  Globe,
  Layers,
  Shield,
  Paintbrush,
  MonitorSmartphone,
  Container,
  Cog,
  Zap,
  Cable,
  HardDrive,
  BarChart3,
  FileCode2,
  GitBranch,
  type LucideIcon,
} from "lucide-react";

interface TechItem {
  name: string;
  description: string;
  version?: string;
  url: string;
}

interface TechCategory {
  title: string;
  icon: LucideIcon;
  color: string;
  items: TechItem[];
}

const categories: TechCategory[] = [
  {
    title: "Backend",
    icon: Server,
    color: "text-orange-500",
    items: [
      { name: "Rust", description: "Systems programming language — safe, fast, concurrent", version: "2024 Edition", url: "https://www.rust-lang.org" },
      { name: "Actix Web", description: "High-performance async HTTP framework", version: "4", url: "https://actix.rs" },
      { name: "Tokio", description: "Async runtime for Rust", version: "1", url: "https://tokio.rs" },
      { name: "Tonic", description: "gRPC framework built on Hyper and Tower", version: "0.12", url: "https://github.com/hyperium/tonic" },
      { name: "Prost", description: "Protocol Buffers implementation for Rust", version: "0.13", url: "https://github.com/tokio-rs/prost" },
      { name: "Utoipa", description: "OpenAPI / Swagger auto-generation", version: "5", url: "https://github.com/juhaku/utoipa" },
    ],
  },
  {
    title: "Data & Storage",
    icon: Database,
    color: "text-blue-500",
    items: [
      { name: "PostgreSQL", description: "Primary relational database (via sqlx 0.8)", version: "16", url: "https://www.postgresql.org" },
      { name: "Redis", description: "Caching, state, and task queue (Redis Streams in DEV)", version: "7", url: "https://redis.io" },
      { name: "Apache Kafka", description: "Distributed event streaming (PROD task queue via rdkafka)", version: "0.36", url: "https://kafka.apache.org" },
      { name: "RustFS", description: "S3-compatible object storage (replaces MinIO)", url: "https://github.com/rustfs/rustfs" },
    ],
  },
  {
    title: "Frontend",
    icon: MonitorSmartphone,
    color: "text-green-500",
    items: [
      { name: "React", description: "Declarative component-based UI library", version: "19", url: "https://react.dev" },
      { name: "TypeScript", description: "Typed superset of JavaScript", version: "5.9", url: "https://www.typescriptlang.org" },
      { name: "Vite", description: "Next-generation frontend build tool (SWC)", version: "7", url: "https://vite.dev" },
      { name: "React Router", description: "Client-side routing", version: "7", url: "https://reactrouter.com" },
      { name: "TanStack Query", description: "Async state management and data fetching", version: "5", url: "https://tanstack.com/query" },
      { name: "TanStack Table", description: "Headless table logic for powerful data tables", version: "8", url: "https://tanstack.com/table" },
    ],
  },
  {
    title: "UI & Styling",
    icon: Paintbrush,
    color: "text-purple-500",
    items: [
      { name: "Tailwind CSS", description: "Utility-first CSS framework", version: "4", url: "https://tailwindcss.com" },
      { name: "Radix UI", description: "Headless, accessible component primitives", url: "https://www.radix-ui.com" },
      { name: "Lucide React", description: "Beautiful open-source icon library", url: "https://lucide.dev" },
      { name: "Recharts", description: "Composable chart library built on D3", version: "2", url: "https://recharts.org" },
      { name: "React Flow", description: "Interactive node-based workflow diagrams", version: "12", url: "https://reactflow.dev" },
      { name: "Sonner", description: "Opinionated toast notification library", version: "2", url: "https://sonner.emilkowal.dev" },
      { name: "CVA", description: "Class Variance Authority — variant-driven styling", url: "https://cva.style" },
    ],
  },
  {
    title: "Infrastructure",
    icon: Container,
    color: "text-cyan-500",
    items: [
      { name: "Docker", description: "Containerized builds and deployments", url: "https://www.docker.com" },
      { name: "Docker Compose", description: "Multi-container orchestration for DEV and PROD", url: "https://docs.docker.com/compose" },
      { name: "Nginx", description: "Reverse proxy, static file server, and load balancer", url: "https://nginx.org" },
      { name: "PgBouncer", description: "Lightweight PostgreSQL connection pooler (PROD)", url: "https://www.pgbouncer.org" },
    ],
  },
  {
    title: "Observability",
    icon: BarChart3,
    color: "text-yellow-500",
    items: [
      { name: "Seq", description: "Structured log server for centralized logging (PROD)", url: "https://datalust.co/seq" },
      { name: "Tracing", description: "Rust instrumentation framework (tracing + tracing-subscriber)", url: "https://tracing.rs" },
    ],
  },
  {
    title: "Protocols & APIs",
    icon: Cable,
    color: "text-pink-500",
    items: [
      { name: "gRPC", description: "High-performance RPC with Protocol Buffers", url: "https://grpc.io" },
      { name: "REST / JSON", description: "Conductor-compatible HTTP API with Swagger UI", url: "https://swagger.io" },
      { name: "Redis Streams", description: "Lightweight task queue protocol for DEV mode", url: "https://redis.io/docs/data-types/streams" },
    ],
  },
];

const highlights = [
  { icon: Zap, label: "Conductor-Compatible", desc: "Drop-in replacement for Netflix Conductor" },
  { icon: Shield, label: "Rust-Powered", desc: "Memory-safe, zero-cost abstractions, fearless concurrency" },
  { icon: Layers, label: "Dual Queue Mode", desc: "Redis Streams (DEV) or Kafka (PROD) via feature flags" },
  { icon: HardDrive, label: "S3 Storage", desc: "RustFS / S3-compatible external payload storage" },
  { icon: Globe, label: "Multi-Protocol", desc: "REST + gRPC endpoints with Swagger docs" },
  { icon: GitBranch, label: "Open Stack", desc: "100% open-source technologies" },
  { icon: Cog, label: "Feature Flags", desc: "Compile-time feature flags for kafka, seq, external-storage" },
  { icon: FileCode2, label: "9 UI Themes", desc: "Default, Warcraft, Cyberpunk, Forest, Ocean, Pokémon, Yu-Gi-Oh!, Chuck Norris, LOTR" },
];

export default function About() {
  const t = useThemeText();

  return (
    <div className="space-y-8">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold tracking-tight">{t.aboutTitle}</h1>
        <p className="text-muted-foreground mt-1">{t.aboutSubtitle}</p>
      </div>

      {/* Highlights */}
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {highlights.map((h) => (
          <Card key={h.label} className="group hover:shadow-md transition-shadow">
            <CardContent className="p-4 flex items-start gap-3">
              <div className="rounded-lg bg-primary/10 p-2 shrink-0 group-hover:bg-primary/20 transition-colors">
                <h.icon className="h-5 w-5 text-primary" />
              </div>
              <div className="min-w-0">
                <p className="text-sm font-semibold leading-tight">{h.label}</p>
                <p className="text-xs text-muted-foreground mt-0.5">{h.desc}</p>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Tech categories */}
      <div className="grid gap-6 md:grid-cols-2">
        {categories.map((cat) => (
          <Card key={cat.title}>
            <CardHeader className="pb-3">
              <CardTitle className="flex items-center gap-2 text-base">
                <cat.icon className={`h-5 w-5 ${cat.color}`} />
                {cat.title}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {cat.items.map((item) => (
                <a
                  key={item.name}
                  href={item.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-start gap-3 rounded-md p-2 -mx-2 hover:bg-accent transition-colors group"
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium group-hover:text-primary transition-colors">
                        {item.name}
                      </span>
                      {item.version && (
                        <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
                          v{item.version}
                        </Badge>
                      )}
                    </div>
                    <p className="text-xs text-muted-foreground mt-0.5 leading-relaxed">
                      {item.description}
                    </p>
                  </div>
                </a>
              ))}
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Footer */}
      <div className="text-center text-xs text-muted-foreground pb-4">
        {t.aboutFooter}
      </div>
    </div>
  );
}
