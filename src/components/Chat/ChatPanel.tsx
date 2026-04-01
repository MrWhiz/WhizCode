import React from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { vscDarkPlus } from "react-syntax-highlighter/dist/esm/styles/prism";
import type { Message, AgentStep } from "../../types";
import { ChatSettings } from "./ChatSettings";
import { MermaidDiagram } from "./MermaidDiagram";
import { SpecPlanPanel } from "./SpecPlanPanel";
import { StreamingDisplay } from "./StreamingDisplay";
import { TerminalBlock } from "./TerminalBlock";
import { TaskClarificationPanel } from "./TaskClarificationPanel";
import { LoopRecoveryPanel } from "./LoopRecoveryPanel";
import { ConfidencePanel } from "./ConfidencePanel";
import { ReasoningPanel } from "./ReasoningPanel";
import { ContextIntegrationPanel } from "./ContextIntegrationPanel";
import type { ExecutionPlanSnapshot, TaskSnapshot } from "../../lib/tauri-api";

interface ChatPanelProps {
  chatWidth: number;
  handleChatResize: (e: React.MouseEvent) => void;
  isChatOpen: boolean;
  setIsChatOpen: (open: boolean) => void;
  workspacePath: string | null;
  messages: Message[];
  isLoading: boolean;
  agentSteps: AgentStep[];
  input: string;
  setInput: (val: string) => void;
  handleSend: () => void;
  handleReset: () => void;
  handleKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  getToolIcon: (tool: string) => string;
  messagesEndRef: React.RefObject<HTMLDivElement | null>;
  handlePermissionResponse: (approved: boolean, stepIdx?: number) => void;
  handleStop: () => void;
  // Settings props
  settingsProps: any;
  liveStreamingContent?: string;
  selectedImages: string[];
  setSelectedImages: React.Dispatch<React.SetStateAction<string[]>>;
  currentPlan?: ExecutionPlanSnapshot | null;
  activeSpec?: any | null;
  taskSnapshot?: TaskSnapshot | null;
}

const LogContainer = ({ logs }: { logs: string[] }) => {
  const [expanded, setExpanded] = React.useState(false);
  const logsEndRef = React.useRef<HTMLDivElement>(null);
  const maxCollapsedLogs = 120;

  React.useEffect(() => {
    logsEndRef.current?.scrollIntoView({
      behavior: "smooth",
      block: "nearest",
    });
  }, [logs]);

  const visibleLogs =
    expanded || logs.length <= maxCollapsedLogs
      ? logs
      : [
          ...logs.slice(0, 80),
          `... ${logs.length - 120} more log lines hidden ...`,
          ...logs.slice(-39),
        ];

  return (
    <div className="agent-step-logs">
      {visibleLogs.map((log, li) => (
        <span
          key={`${li}-${log.slice(0, 24)}`}
          className="log-line"
          style={
            log.startsWith("... ")
              ? { opacity: 0.7, fontStyle: "italic" }
              : undefined
          }
        >
          {log}
        </span>
      ))}
      {logs.length > maxCollapsedLogs && (
        <button
          type="button"
          onClick={() => setExpanded((prev) => !prev)}
          style={{
            marginTop: "6px",
            background: "transparent",
            border: "1px solid var(--border-color)",
            borderRadius: "4px",
            color: "var(--text-secondary)",
            padding: "4px 8px",
            fontSize: "10px",
            cursor: "pointer",
            alignSelf: "flex-start",
          }}
        >
          {expanded ? "Show less" : `Show all ${logs.length} lines`}
        </button>
      )}
      <div ref={logsEndRef} />
    </div>
  );
};

const isHighRiskPermissionSummary = (summary: string): boolean => {
  const normalized = summary.toLowerCase();
  return [
    "rm -rf",
    "remove-item -recurse -force",
    "git reset --hard",
    "git checkout --",
    "del /f",
    "rmdir /s",
    "format ",
    "drop database",
    "truncate table",
  ].some((pattern) => normalized.includes(pattern));
};

// Smart summary formatter — turns raw "Executed X with args: {...}" into readable text
const formatStepSummary = (tool: string, summary: string): string => {
  const normalized = summary.toLowerCase();
  if (normalized.includes("already read")) {
    if (tool === "read_file" || tool === "view_structure") {
      const fileMatch = summary.match(
        /([A-Za-z0-9._-]+(?:\.[A-Za-z0-9._-]+)?)(?:\s*\]|$)/,
      );
      if (fileMatch?.[1]) {
        return `Already read ${fileMatch[1]}`;
      }
    }
    return "Already read this file";
  }
  // Try to extract args JSON from the summary
  const match = summary.match(/args:\s*(\{.*\})/s);
  if (!match) return summary;

  try {
    const args = JSON.parse(match[1]);
    switch (tool) {
      case "write_file":
      case "edit_file": {
        const path = args.path || args.file || "";
        const fileName = path.split(/[/\\]/).pop() || path;
        return tool === "write_file"
          ? `Write  ${fileName}`
          : `Edit  ${fileName}${args.start_line ? `  (lines ${args.start_line}–${args.end_line || "?"})` : ""}`;
      }
      case "read_file": {
        const path = args.path || "";
        const fileName = path.split(/[/\\]/).pop() || path;
        return `Read  ${fileName}`;
      }
      case "list_directory": {
        const path = args.path || "";
        return `List  ${path}`;
      }
      case "search_files": {
        return `Search  "${args.pattern || args.query}"${args.path ? `  in ${args.path}` : ""}`;
      }
      case "run_command": {
        return `$ ${args.command || ""}`;
      }
      case "git": {
        return `git ${args.operation || ""}${args.message ? ` "${args.message}"` : ""}`;
      }
      case "npm": {
        return `npm ${args.operation || ""}${args.package ? ` ${args.package}` : ""}`;
      }
      default:
        return summary.replace(/Executed \w+ with args: \{.*\}/s, `${tool}`);
    }
  } catch {
    return summary;
  }
};

const StepBlock = ({
  step,
  getToolIcon,
  isLive = false,
}: {
  step: AgentStep;
  getToolIcon: (t: string) => string;
  isLive?: boolean;
}) => {
  const [logsOpen, setLogsOpen] = React.useState(false);
  const hasLogs = step.logs && step.logs.length > 0;
  const canOpenLogs = step.tool === "run_command" || hasLogs;

  // Auto-open logs if the step is a running command or if it fails
  React.useEffect(() => {
    if (
      (isLive &&
        (step.status === "running" || step.status === "started") &&
        step.tool === "run_command") ||
      step.status === "failed"
    ) {
      setLogsOpen(true);
    }
  }, [isLive, step.status, step.tool]);

  const personaIcon =
    step.persona === "planner"
      ? "🗺️"
      : step.persona === "researcher"
        ? "🔍"
        : step.persona === "executor"
          ? "🛠️"
          : step.persona === "reviewer"
            ? "⚖️"
            : "🤖";
  const personaColor =
    step.persona === "planner"
      ? "#cba6f7"
      : step.persona === "researcher"
        ? "#89b4fa"
        : step.persona === "executor"
          ? "#a6e3a1"
          : step.persona === "reviewer"
            ? "#f9e2af"
            : "#9399b2";

  const handleClick = () => {
    if (canOpenLogs) {
      setLogsOpen((o) => !o);
    }
  };

  const statusAccent =
    step.status === "failed"
      ? "#f38ba8"
      : step.status === "completed" || step.status === "done"
        ? "#a6e3a1"
        : step.status === "started"
          ? "#89b4fa"
          : step.status === "identified"
            ? "#6c7086"
            : "var(--accent-primary)";

  const displaySummary = formatStepSummary(step.tool, step.summary);
  const liveProgressText =
    isLive && (step.status === "running" || step.status === "started")
      ? step.tool === "run_command"
        ? "Streaming command output…"
        : step.tool === "read_file"
          ? "Reading file…"
          : step.tool === "semantic_search"
            ? "Searching workspace…"
            : step.tool === "search_files"
              ? "Scanning filenames…"
              : "Working…"
      : null;

  return (
    <div
      className={`agent-step ${step.status}`}
      style={{
        border: "1px solid rgba(255,255,255,0.08)",
        borderRadius: "12px",
        background: "rgba(255,255,255,0.03)",
        overflow: "hidden",
        boxShadow: "0 8px 20px rgba(0,0,0,0.12)",
      }}
    >
      <div
        className="agent-step-header"
        onClick={handleClick}
        style={{
          cursor: canOpenLogs ? "pointer" : "default",
          userSelect: "none",
          display: "flex",
          alignItems: "center",
          gap: "8px",
          padding: "10px 12px",
          minHeight: "44px",
          background: `linear-gradient(90deg, ${statusAccent}15, rgba(255,255,255,0.02))`,
        }}
      >
        {step.persona && (
          <span
            style={{
              fontSize: "9px",
              background: `${personaColor}15`,
              color: personaColor,
              padding: "2px 6px",
              borderRadius: "999px",
              border: `1px solid ${personaColor}33`,
              fontWeight: 700,
              display: "flex",
              alignItems: "center",
              gap: "3px",
            }}
          >
            {personaIcon} {step.persona.toUpperCase()}
          </span>
        )}
        {isLive && (step.status === "running" || step.status === "started") ? (
          <div className="spinner" style={{ width: 10, height: 10 }}></div>
        ) : (
          <span className="agent-step-icon">{getToolIcon(step.tool)}</span>
        )}
        <span
          className="agent-step-summary"
          style={{
            fontSize: "12px",
            lineHeight: "1.45",
            color: "var(--text-primary)",
            flex: 1,
            minWidth: 0,
            wordBreak: "break-word",
          }}
        >
          {displaySummary}
        </span>
        {/* Status badge */}
        <span
          style={{
            marginLeft: "auto",
            fontSize: "9px",
            padding: "3px 8px",
            borderRadius: "999px",
            fontWeight: 800,
            backgroundColor:
              step.status === "identified"
                ? "#6c7086"
                : step.status === "started"
                  ? "#89b4fa"
                  : step.status === "completed" || step.status === "done"
                    ? "#a6e3a1"
                    : step.status === "failed"
                      ? "#f38ba8"
                      : step.status === "skipped"
                        ? "#f9e2af"
                        : "#9399b2",
            color: "#1e1e2e",
          }}
        >
          {step.status === "identified"
            ? "IDENTIFIED"
            : step.status === "started"
              ? "RUNNING"
              : step.status === "completed"
                ? "DONE"
                : step.status === "done"
                  ? "DONE"
                  : step.status === "failed"
                    ? "FAILED"
                    : step.status === "skipped"
                      ? "SKIPPED"
                      : step.status.toUpperCase()}
        </span>
        {(step.status === "done" || step.status === "completed") && (
          <span className="agent-step-check">✓</span>
        )}
        {step.status === "failed" && (
          <span style={{ color: "var(--error-color)", fontSize: 12 }}>✗</span>
        )}
        {canOpenLogs && (
          <span style={{ fontSize: "10px", opacity: 0.5, paddingLeft: 2 }}>
            {logsOpen ? "▲" : "▼"}
          </span>
        )}
      </div>
      {liveProgressText && (
        <div
          style={{
            padding: "0 12px 8px 12px",
            display: "flex",
            alignItems: "center",
            gap: "8px",
            fontSize: "11px",
            color: "var(--text-secondary)",
          }}
        >
          <span
            className="spinner"
            style={{ width: 8, height: 8, flexShrink: 0 }}
          />
          <span
            style={{ display: "inline-flex", alignItems: "center", gap: "4px" }}
          >
            <span>{liveProgressText}</span>
            <span
              style={{
                display: "inline-flex",
                gap: "2px",
                transform: "translateY(-1px)",
              }}
            >
              <span
                style={{
                  animation: "blink 1s infinite",
                  animationDelay: "0ms",
                }}
              >
                •
              </span>
              <span
                style={{
                  animation: "blink 1s infinite",
                  animationDelay: "180ms",
                }}
              >
                •
              </span>
              <span
                style={{
                  animation: "blink 1s infinite",
                  animationDelay: "360ms",
                }}
              >
                •
              </span>
            </span>
          </span>
        </div>
      )}
      {step.data && <EditDetails data={step.data} />}
      {step.result && step.result.includes("file:///") && (
        <div style={{ padding: "8px 12px" }}>
          <img
            src={
              step.result.split("URL: ")[1] ||
              step.result.match(/file:\/\/\/[^\s]+/)?.[0]
            }
            alt="Generated Asset"
            style={{
              maxWidth: "100%",
              borderRadius: "4px",
              border: "1px solid #313244",
              cursor: "pointer",
            }}
            onClick={() => window.open(step.result?.split("URL: ")[1])}
          />
        </div>
      )}
      {step.result &&
        !step.result.includes("file:///") &&
        (step.status === "failed" || step.status === "skipped") && (
          <div
            style={{
              margin: step.status === "skipped" ? "0 12px 10px" : "0 12px 12px",
              padding: step.status === "skipped" ? "6px 10px" : "8px 12px",
              fontSize: step.status === "skipped" ? "11px" : "12px",
              lineHeight: 1.45,
              color:
                step.status === "skipped" ? "#f9e2af" : "var(--error-color)",
              background:
                step.status === "skipped"
                  ? "rgba(249, 226, 175, 0.08)"
                  : "transparent",
              border:
                step.status === "skipped"
                  ? "1px solid rgba(249, 226, 175, 0.18)"
                  : "none",
              borderRadius: step.status === "skipped" ? "8px" : 0,
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
            }}
          >
            {step.result}
          </div>
        )}
      {canOpenLogs &&
        logsOpen &&
        (step.tool === "run_command" ? (
          <TerminalBlock
            logs={step.logs && step.logs.length > 0 ? step.logs : []}
            isLive={isLive}
            isRunning={step.status === "running" || step.status === "started"}
            requestId={step.requestId}
          />
        ) : (
          <LogContainer
            logs={
              step.logs && step.logs.length > 0 ? step.logs : ["(No logs yet)"]
            }
          />
        ))}
    </div>
  );
};

const FileActionSummary = ({ files }: { files: any[] }) => {
  if (!Array.isArray(files) || files.length === 0) return null;

  const actionLabel = (file: any) => {
    switch (file.action) {
      case "created":
        return "Created";
      case "created_dir":
        return "Created folder";
      case "deleted":
        return "Deleted";
      case "moved":
        return "Moved";
      case "read":
        return "Read";
      default:
        return "Edited";
    }
  };

  return (
    <div
      style={{
        marginTop: "8px",
        display: "flex",
        flexDirection: "column",
        gap: "6px",
      }}
    >
      {files.map((file, index) => {
        const fileName =
          (file.path || "").split(/[/\\]/).pop() || file.path || "file";
        const hasDelta =
          typeof file.added === "number" || typeof file.removed === "number";
        return (
          <div
            key={`${file.path || "file"}-${index}`}
            style={{
              display: "flex",
              alignItems: "center",
              gap: "8px",
              fontSize: "12px",
              paddingLeft: "18px",
              flexWrap: "wrap",
            }}
          >
            <span style={{ color: "var(--text-secondary)" }}>
              {actionLabel(file)}
            </span>
            <span
              style={{
                color: "var(--accent-primary)",
                textDecoration: "underline",
              }}
            >
              {fileName}
            </span>
            {file.startLine && (
              <span style={{ color: "var(--text-tertiary)" }}>
                lines {file.startLine}-{file.endLine || file.startLine}
              </span>
            )}
            {file.from && file.to && (
              <span style={{ color: "var(--text-tertiary)" }}>
                {file.from.split(/[/\\]/).pop()} →{" "}
                {file.to.split(/[/\\]/).pop()}
              </span>
            )}
            {hasDelta && (
              <>
                <span style={{ color: "#22c55e" }}>+{file.added || 0}</span>
                <span style={{ color: "#ef4444" }}>-{file.removed || 0}</span>
              </>
            )}
          </div>
        );
      })}
    </div>
  );
};

const EditDetails = ({ data }: { data: any }) => {
  const [isOpen, setIsOpen] = React.useState(false);

  if (!data) return null;
  const hasExpandableChanges = Boolean(data.edits || data.changes);

  // Handle plan data
  if (data.plan) {
    const plan = data.plan;
    return (
      <div className="agent-step-details" style={{ marginTop: "8px" }}>
        <div
          className="details-toggle"
          onClick={() => setIsOpen(!isOpen)}
          style={{
            fontSize: "11px",
            color: "var(--accent-primary)",
            cursor: "pointer",
            display: "flex",
            alignItems: "center",
            gap: "4px",
            userSelect: "none",
            fontWeight: 600,
          }}
        >
          {isOpen ? "⊖ Hide Plan" : "⊕ View Plan"}
        </div>
        {isOpen && (
          <div
            className="details-content"
            style={{
              marginTop: "8px",
              background: "rgba(0,0,0,0.2)",
              borderRadius: "4px",
              overflow: "hidden",
              border: "1px solid var(--border-color)",
              padding: "8px",
            }}
          >
            <div style={{ marginBottom: "8px" }}>
              <div
                style={{
                  fontSize: "10px",
                  color: "var(--text-tertiary)",
                  fontWeight: 600,
                  textTransform: "uppercase",
                  marginBottom: "4px",
                }}
              >
                Objective
              </div>
              <div style={{ fontSize: "11px", color: "var(--text-primary)" }}>
                {plan.objective}
              </div>
            </div>

            <div style={{ marginBottom: "8px" }}>
              <div
                style={{
                  fontSize: "10px",
                  color: "var(--text-tertiary)",
                  fontWeight: 600,
                  textTransform: "uppercase",
                  marginBottom: "4px",
                }}
              >
                Tasks ({plan.tasks?.length || 0})
              </div>
              <div
                style={{ display: "flex", flexDirection: "column", gap: "4px" }}
              >
                {plan.tasks?.map((task: any, i: number) => (
                  <div
                    key={i}
                    style={{
                      fontSize: "10px",
                      padding: "4px 6px",
                      background: "rgba(0,0,0,0.3)",
                      borderRadius: "3px",
                      borderLeft: "2px solid var(--accent-primary)",
                    }}
                  >
                    <div
                      style={{ fontWeight: 600, color: "var(--text-primary)" }}
                    >
                      {i + 1}. {task.description}
                    </div>
                    <div
                      style={{
                        fontSize: "9px",
                        color: "var(--text-secondary)",
                        marginTop: "2px",
                      }}
                    >
                      Type: {task.type} • Duration: ~{task.estimatedDuration}s
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <div
              style={{
                display: "grid",
                gridTemplateColumns: "1fr 1fr",
                gap: "8px",
                fontSize: "10px",
              }}
            >
              <div>
                <div
                  style={{
                    color: "var(--text-tertiary)",
                    fontWeight: 600,
                    textTransform: "uppercase",
                    marginBottom: "2px",
                  }}
                >
                  Duration
                </div>
                <div style={{ color: "var(--text-primary)" }}>
                  ~{plan.estimatedDuration}s
                </div>
              </div>
              <div>
                <div
                  style={{
                    color: "var(--text-tertiary)",
                    fontWeight: 600,
                    textTransform: "uppercase",
                    marginBottom: "2px",
                  }}
                >
                  Risk Level
                </div>
                <div
                  style={{
                    color:
                      plan.riskLevel === "high"
                        ? "#ff6b6b"
                        : plan.riskLevel === "medium"
                          ? "#ffa500"
                          : "#51cf66",
                  }}
                >
                  {plan.riskLevel?.toUpperCase()}
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="agent-step-details" style={{ marginTop: "8px" }}>
      {data.files && <FileActionSummary files={data.files} />}
      {hasExpandableChanges && (
        <div
          className="details-toggle"
          onClick={() => setIsOpen(!isOpen)}
          style={{
            fontSize: "11px",
            color: "var(--accent-primary)",
            cursor: "pointer",
            display: "flex",
            alignItems: "center",
            gap: "4px",
            userSelect: "none",
            fontWeight: 600,
          }}
        >
          {isOpen ? "⊖ Hide Changes" : "⊕ View Changes"}
        </div>
      )}
      {hasExpandableChanges && isOpen && (
        <div
          className="details-content"
          style={{
            marginTop: "8px",
            background: "rgba(0,0,0,0.2)",
            borderRadius: "4px",
            overflow: "hidden",
            border: "1px solid var(--border-color)",
          }}
        >
          {data.edits
            ? data.edits.map((edit: any, i: number) => (
                <div
                  key={i}
                  className="edit-block-preview"
                  style={{
                    padding: "8px",
                    borderBottom:
                      i < data.edits.length - 1
                        ? "1px solid var(--border-color)"
                        : "none",
                  }}
                >
                  <div
                    style={{
                      fontSize: "10px",
                      color: "#f14c4c",
                      fontWeight: 600,
                      marginBottom: "4px",
                    }}
                  >
                    REMOVE
                  </div>
                  <pre
                    style={{
                      margin: "0 0 8px 0",
                      fontSize: "11px",
                      whiteSpace: "pre-wrap",
                      color: "#ff8888",
                      background: "rgba(241,76,76,0.05)",
                      padding: "4px",
                      borderRadius: "2px",
                    }}
                  >
                    {edit.search}
                  </pre>
                  <div
                    style={{
                      fontSize: "10px",
                      color: "#89d185",
                      fontWeight: 600,
                      marginBottom: "4px",
                    }}
                  >
                    ADD
                  </div>
                  <pre
                    style={{
                      margin: 0,
                      fontSize: "11px",
                      whiteSpace: "pre-wrap",
                      color: "#89d185",
                      background: "rgba(137,209,133,0.05)",
                      padding: "4px",
                      borderRadius: "2px",
                    }}
                  >
                    {edit.replace}
                  </pre>
                </div>
              ))
            : data.changes
              ? data.changes.map((change: any, i: number) => (
                  <div
                    key={i}
                    className="edit-block-preview"
                    style={{
                      padding: "8px",
                      borderBottom:
                        i < data.changes.length - 1
                          ? "1px solid var(--border-color)"
                          : "none",
                    }}
                  >
                    <div
                      style={{
                        fontSize: "11px",
                        fontWeight: 600,
                        marginBottom: "6px",
                        color: "var(--text-secondary)",
                      }}
                    >
                      {change.path}
                    </div>
                    <pre
                      style={{
                        margin: 0,
                        fontSize: "11px",
                        whiteSpace: "pre-wrap",
                        fontFamily: "var(--font-mono)",
                        color: "var(--text-primary)",
                      }}
                    >
                      {change.diff}
                    </pre>
                  </div>
                ))
              : null}
        </div>
      )}
    </div>
  );
};

const MessageContent = ({
  content,
  role,
}: {
  content: string;
  role: string;
}) => {
  if (role !== "assistant") return <>{content}</>;

  let thoughts: string[] = [];
  let cleanContent = content;

  // Extract thoughts from JSON format: {"thought": "...", "tool": "...", "args": {...}}
  // Look for JSON objects with "thought" key
  const jsonThoughtRegex = /"thought"\s*:\s*"([^"\\]*(?:\\.[^"\\]*)*)"/g;
  let match;
  while ((match = jsonThoughtRegex.exec(content)) !== null) {
    const thought = match[1].replace(/\\"/g, '"').replace(/\\\\/g, "\\").trim();
    if (thought && !thoughts.includes(thought)) {
      thoughts.push(thought);
    }
  }

  // Remove JSON tool call blocks from display
  cleanContent = content
    .replace(/```json\s*\{[\s\S]*?\}\s*```/g, "")
    .replace(/(?:\n|^)\s*\{[\s\S]*?"tool"\s*:[\s\S]*?\}\s*(?:\n|$)/g, "")
    .trim();

  // Normalize and strip all possible internal control tags
  cleanContent = cleanContent
    .replace(/<IDENTITY>[\s\S]*?<\/IDENTITY>/gi, "")
    .replace(/<PRIME_DIRECTIVE>[\s\S]*?<\/PRIME_DIRECTIVE>/gi, "")
    .replace(/<PLAN>[\s\S]*?<\/PLAN>/gi, "")
    .replace(/<PROJECT_STATUS>[\s\S]*?<\/PROJECT_STATUS>/gi, "")
    .replace(/<OUTPUT_FORMAT>[\s\S]*?<\/OUTPUT_FORMAT>/gi, "")
    .trim();

  // Define regex patterns for JSON blocks
  const jsonBlockRegex = /```json\s*\{[\s\S]*?\}\s*```/g;
  const rawJsonRegex = /(?:\n|^)\s*\{[\s\S]*?\}\s*(?:\n|$)/g;
  const streamingJsonBlockRegex = /```json\s*\{[\s\S]*$/g;
  const streamingRawJsonRegex = /(?:\n|^)\s*\{[\s\S]*$/g;

  // Check if the content is JUST a tool call (no other text or thoughts)
  const hasToolCallJson =
    cleanContent.includes('"tool":') ||
    (cleanContent.includes("{") && cleanContent.includes('"tool"'));

  // Strip JSON blocks (complete and partial)
  let strippedContent = cleanContent
    .replace(jsonBlockRegex, "")
    .replace(rawJsonRegex, "")
    .replace(streamingJsonBlockRegex, "")
    .replace(streamingRawJsonRegex, "")
    .trim();

  // If there's literally nothing left but a tool call and no thoughts, hide the message body
  if (hasToolCallJson && !strippedContent && thoughts.length === 0) {
    return null;
  }

  // Otherwise, show the stripped content (the explanation/reasoning) but hide the JSON blocks
  const finalDisplayContent = strippedContent;

  // 3. Detect if the remaining content is just a JSON-like completion summary
  let isJsonSummary = false;
  if (
    finalDisplayContent.startsWith("{") &&
    finalDisplayContent.endsWith("}") &&
    finalDisplayContent.includes('"status"')
  ) {
    try {
      isJsonSummary = true;
    } catch (e) {}
  }

  return (
    <div className="assistant-content-wrapper">
      {thoughts.length > 0 && (
        <div
          className="thought-process glass"
          style={{
            marginBottom: "12px",
            borderRadius: "6px",
            borderLeft: "3px solid var(--accent-primary)",
            padding: "8px 10px",
          }}
        >
          <div
            className="thought-header"
            style={{
              fontSize: "10px",
              fontWeight: 700,
              color: "var(--text-tertiary)",
              textTransform: "uppercase",
              letterSpacing: "0.5px",
              marginBottom: "4px",
              display: "flex",
              alignItems: "center",
              gap: "6px",
            }}
          >
            <span style={{ opacity: 0.6 }}>🧠</span> REASONING
          </div>
          {thoughts.map((t, i) => (
            <div
              key={i}
              className="thought-body"
              style={{
                fontSize: "11.5px",
                color: "var(--text-secondary)",
                fontStyle: "italic",
                lineHeight: "1.4",
              }}
            >
              {t}
            </div>
          ))}
        </div>
      )}
      <div className="message-main-body">
        {isJsonSummary ? (
          <SyntaxHighlighter
            style={vscDarkPlus as any}
            language="json"
            PreTag="div"
            customStyle={{
              margin: "8px 0",
              borderRadius: "6px",
              fontSize: "12px",
              border: "1px solid var(--border-color)",
              background: "rgba(0,0,0,0.3)",
            }}
          >
            {finalDisplayContent}
          </SyntaxHighlighter>
        ) : (
          <ReactMarkdown
            remarkPlugins={[remarkGfm]}
            components={{
              code({ className, children, ...props }) {
                const match = /language-(\w+)/.exec(className || "");
                const codeString = String(children).replace(/\n$/, "");

                if (match && match[1] === "mermaid") {
                  return <MermaidDiagram chart={codeString} />;
                }

                return match ? (
                  <SyntaxHighlighter
                    style={vscDarkPlus as any}
                    language={match[1]}
                    PreTag="div"
                    customStyle={{
                      margin: "8px 0",
                      borderRadius: "6px",
                      fontSize: "12px",
                      border: "1px solid var(--border-color)",
                    }}
                  >
                    {codeString}
                  </SyntaxHighlighter>
                ) : (
                  <code className="inline-code" {...props}>
                    {children}
                  </code>
                );
              },
              a({ href, children }) {
                return (
                  <a
                    href={href}
                    target="_blank"
                    rel="noreferrer"
                    className="md-link"
                  >
                    {children}
                  </a>
                );
              },
            }}
          >
            {finalDisplayContent}
          </ReactMarkdown>
        )}
      </div>
    </div>
  );
};

const ArchivedMessagesList = React.memo(
  ({
    messages,
    getToolIcon,
  }: {
    messages: Message[];
    getToolIcon: (tool: string) => string;
  }) => {
    return (
      <>
        {messages.map((msg, idx) => (
          <div key={idx} className={`chat-msg ${msg.role}`}>
            <div className="chat-msg-sender">
              {msg.role === "user" ? "YOU" : "WhizCode"}
            </div>
            {msg.steps && msg.steps.length > 0 && (
              <div className="agent-steps">
                {msg.steps.map((step, si) => (
                  <StepBlock key={si} step={step} getToolIcon={getToolIcon} />
                ))}
              </div>
            )}
            <div className="chat-msg-content">
              {msg.images && msg.images.length > 0 && (
                <div
                  className="msg-images"
                  style={{
                    display: "flex",
                    flexWrap: "wrap",
                    gap: "8px",
                    marginBottom: "8px",
                  }}
                >
                  {msg.images.map((img, i) => (
                    <img
                      key={i}
                      src={img}
                      style={{
                        maxWidth: "200px",
                        maxHeight: "150px",
                        borderRadius: "4px",
                        border: "1px solid rgba(255,255,255,0.1)",
                      }}
                    />
                  ))}
                </div>
              )}
              <MessageContent content={msg.content} role={msg.role} />
            </div>
          </div>
        ))}
      </>
    );
  },
  (prev, next) =>
    prev.messages === next.messages && prev.getToolIcon === next.getToolIcon,
);

const LiveAgentActivity = React.memo(
  ({
    liveStreamingContent,
    agentSteps,
    getToolIcon,
  }: {
    liveStreamingContent: string;
    agentSteps: AgentStep[];
    getToolIcon: (tool: string) => string;
  }) => {
    if (!liveStreamingContent && agentSteps.length === 0) {
      return (
        <div className="thinking-indicator">
          <div className="thinking-dot"></div>
          <div className="thinking-dot"></div>
          <div className="thinking-dot"></div>
        </div>
      );
    }

    return (
      <>
        {liveStreamingContent && (
          <StreamingDisplay content={liveStreamingContent} isStreaming={true} />
        )}
        {agentSteps.length > 0 && (
          <div
            className="agent-steps live"
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "8px",
              marginTop: "10px",
              paddingTop: "6px",
            }}
          >
            {agentSteps.map((step, si) => (
              <StepBlock
                key={step.requestId || `step_${si}`}
                step={step}
                getToolIcon={getToolIcon}
                isLive={true}
              />
            ))}
          </div>
        )}
      </>
    );
  },
  (prev, next) =>
    prev.liveStreamingContent === next.liveStreamingContent &&
    prev.agentSteps === next.agentSteps &&
    prev.getToolIcon === next.getToolIcon,
);

const LiveThoughtPanel = React.memo(
  ({
    currentPhase,
    elapsedSeconds,
    liveStreamingContent,
    activeThought,
    agentSteps,
    tokensPerSecond,
    estimatedTimeRemaining,
    totalTokens,
    promptDiagnostics,
  }: {
    currentPhase: string;
    elapsedSeconds: number;
    liveStreamingContent: string;
    activeThought: string | null;
    agentSteps: AgentStep[];
    tokensPerSecond?: number;
    estimatedTimeRemaining?: number;
    totalTokens?: number;
    promptDiagnostics: any | null;
  }) => {
    const statusText = liveStreamingContent
      ? "Streaming reasoning and tool calls..."
      : activeThought ||
        (agentSteps.length > 0
          ? agentSteps.find((s: any) => s.status === "running")?.summary ||
            [...agentSteps].reverse().find((s: any) => s.status === "done")
              ?.summary ||
            "Initiating plan..."
          : "Analyzing context...");

    return (
      <div
        className="thought-stream-container glass"
        style={{
          marginBottom: "12px",
          display: "flex",
          flexDirection: "column",
          gap: "10px",
          padding: "14px",
          borderRadius: "14px",
          border: "1px solid rgba(59, 130, 246, 0.16)",
          background:
            "linear-gradient(180deg, rgba(15, 23, 42, 0.78), rgba(15, 23, 42, 0.58))",
          boxShadow: "0 12px 30px rgba(0, 0, 0, 0.16)",
        }}
      >
        <div
          className="dynamic-thought-bar"
          style={{
            border: "1px solid rgba(59, 130, 246, 0.14)",
            borderRadius: "12px",
            padding: "10px 12px",
            fontSize: "12px",
            color: "var(--text-primary)",
            display: "flex",
            alignItems: "flex-start",
            gap: "10px",
            minHeight: "44px",
            overflow: "hidden",
            background: "rgba(255, 255, 255, 0.03)",
          }}
        >
          <div
            className="thought-pulse"
            style={{
              width: "9px",
              height: "9px",
              marginTop: "4px",
              borderRadius: "50%",
              background: "var(--accent-primary)",
              boxShadow: "0 0 10px var(--accent-primary)",
              flexShrink: 0,
            }}
          ></div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div
              style={{
                fontSize: "10px",
                fontWeight: 700,
                letterSpacing: "0.08em",
                textTransform: "uppercase",
                color: "var(--accent-primary)",
                marginBottom: "4px",
              }}
            >
              {currentPhase.toUpperCase()}
            </div>
            <div
              className="live-thought-text"
              style={{
                fontSize: "12px",
                color: "var(--text-primary)",
                lineHeight: "1.55",
                wordBreak: "break-word",
              }}
            >
              {statusText}
              <span
                style={{ animation: "blink 1s infinite", marginLeft: "2px" }}
              >
                ▌
              </span>
            </div>
          </div>
        </div>

        {liveStreamingContent.trim() && (
          <div
            style={{
              maxHeight: "64px",
              overflow: "hidden",
              borderRadius: "12px",
            }}
          >
            <StreamingDisplay
              content={liveStreamingContent}
              isStreaming={true}
            />
          </div>
        )}

        {(tokensPerSecond !== undefined ||
          estimatedTimeRemaining !== undefined ||
          totalTokens !== undefined) && (
          <div
            style={{
              display: "flex",
              gap: "8px",
              flexWrap: "wrap",
            }}
          >
            {totalTokens !== undefined && (
              <div
                style={{
                  display: "inline-flex",
                  gap: "6px",
                  alignItems: "center",
                  padding: "6px 10px",
                  borderRadius: "999px",
                  fontSize: "10px",
                  color: "var(--text-secondary)",
                  background: "rgba(59, 130, 246, 0.08)",
                  border: "1px solid rgba(59, 130, 246, 0.12)",
                }}
              >
                <span
                  style={{ color: "var(--accent-primary)", fontWeight: "bold" }}
                >
                  📊
                </span>
                <span>{totalTokens} tokens</span>
              </div>
            )}
            {tokensPerSecond !== undefined && (
              <div
                style={{
                  display: "inline-flex",
                  gap: "6px",
                  alignItems: "center",
                  padding: "6px 10px",
                  borderRadius: "999px",
                  fontSize: "10px",
                  color: "var(--text-secondary)",
                  background: "rgba(59, 130, 246, 0.08)",
                  border: "1px solid rgba(59, 130, 246, 0.12)",
                }}
              >
                <span
                  style={{ color: "var(--accent-primary)", fontWeight: "bold" }}
                >
                  ⚡
                </span>
                <span>{tokensPerSecond.toFixed(1)} tok/s</span>
              </div>
            )}
            {estimatedTimeRemaining !== undefined && (
              <div
                style={{
                  display: "inline-flex",
                  gap: "6px",
                  alignItems: "center",
                  padding: "6px 10px",
                  borderRadius: "999px",
                  fontSize: "10px",
                  color: "var(--text-secondary)",
                  background: "rgba(59, 130, 246, 0.08)",
                  border: "1px solid rgba(59, 130, 246, 0.12)",
                }}
              >
                <span
                  style={{ color: "var(--accent-primary)", fontWeight: "bold" }}
                >
                  ⏱
                </span>
                <span>
                  ~
                  {estimatedTimeRemaining < 60
                    ? `${estimatedTimeRemaining}s`
                    : `${Math.floor(estimatedTimeRemaining / 60)}m`}
                </span>
              </div>
            )}
          </div>
        )}

        {promptDiagnostics && (
          <div
            style={{
              display: "flex",
              gap: "8px",
              flexWrap: "wrap",
              padding: "8px 10px",
              borderRadius: "10px",
              background: "rgba(245, 158, 11, 0.08)",
              border: "1px solid rgba(245, 158, 11, 0.18)",
              fontSize: "10px",
              color: "var(--text-secondary)",
            }}
          >
            <div style={{ fontWeight: 700, color: "var(--accent-warning)" }}>
              Prompt trimming
            </div>
            <div>{promptDiagnostics.phase}</div>
            <div>
              {promptDiagnostics.included_messages}/
              {promptDiagnostics.total_messages} kept
            </div>
            <div>{promptDiagnostics.omitted_messages} omitted</div>
          </div>
        )}
      </div>
    );
  },
  (prev, next) =>
    prev.currentPhase === next.currentPhase &&
    prev.elapsedSeconds === next.elapsedSeconds &&
    prev.liveStreamingContent === next.liveStreamingContent &&
    prev.activeThought === next.activeThought &&
    prev.agentSteps === next.agentSteps &&
    prev.tokensPerSecond === next.tokensPerSecond &&
    prev.estimatedTimeRemaining === next.estimatedTimeRemaining &&
    prev.totalTokens === next.totalTokens &&
    prev.promptDiagnostics === next.promptDiagnostics,
);

export const ChatPanel = ({
  chatWidth,
  handleChatResize,
  isChatOpen,
  setIsChatOpen,
  workspacePath,
  messages,
  isLoading,
  agentSteps,
  input,
  setInput,
  handleSend,
  handleReset,
  handleKeyDown,
  getToolIcon,
  messagesEndRef,
  handlePermissionResponse,
  handleStop,
  settingsProps,
  liveStreamingContent = "",
  selectedImages,
  setSelectedImages,
  currentPlan = null,
  activeSpec = null,
  taskSnapshot = null,
}: ChatPanelProps) => {
  const [respondedSteps, setRespondedSteps] = React.useState<
    Record<number, boolean>
  >({});
  const [alwaysRun, setAlwaysRun] = React.useState(false);
  const [countdown, setCountdown] = React.useState<number | null>(null);
  const [currentPhase, setCurrentPhase] = React.useState<string>("analyzing");
  const [phaseStartTime, setPhaseStartTime] = React.useState<number>(
    Date.now(),
  );
  const [elapsedSeconds, setElapsedSeconds] = React.useState<number>(0);

  // WhizCode metrics state
  const [tokensPerSecond, setTokensPerSecond] = React.useState<
    number | undefined
  >();
  const [estimatedTimeRemaining, setEstimatedTimeRemaining] = React.useState<
    number | undefined
  >();
  const [totalTokens, setTotalTokens] = React.useState<number | undefined>();
  const [phaseHistory, setPhaseHistory] = React.useState<string[]>([]);
  const [promptDiagnostics, setPromptDiagnostics] = React.useState<any | null>(
    null,
  );

  // Kiro behavior state
  const [clarificationData, setClarificationData] = React.useState<any | null>(
    null,
  );
  const [loopRecoveryData, setLoopRecoveryData] = React.useState<any | null>(
    null,
  );
  const [confidenceData, setConfidenceData] = React.useState<any | null>(null);
  const [reasoningData, setReasoningData] = React.useState<any | null>(null);
  const [contextIntegrationData, setContextIntegrationData] = React.useState<
    any | null
  >(null);

  React.useEffect(() => {
    const unlistenPhase = (window as any)
      .__TAURI_INVOKE__?.("listen", {
        event: "agent:phase",
        handler: (event: any) => {
          const phase = event.payload?.phase || "analyzing";
          setCurrentPhase(phase);
          setPhaseStartTime(Date.now());
          // Add to phase history
          setPhaseHistory((prev) => {
            const updated = [...prev, phase];
            // Keep only last 5 phases to avoid clutter
            return updated.slice(-5);
          });
        },
      })
      .catch(() => {});

    return () => {
      unlistenPhase?.then((unlisten: any) => unlisten?.());
    };
  }, []);

  // Listen for metrics events
  React.useEffect(() => {
    const unlistenMetrics = (window as any)
      .__TAURI_INVOKE__?.("listen", {
        event: "agent:metrics",
        handler: (event: any) => {
          const metrics = event.payload;
          if (metrics.tokens_per_second !== undefined) {
            setTokensPerSecond(metrics.tokens_per_second);
          }
          if (metrics.estimated_time_remaining !== undefined) {
            setEstimatedTimeRemaining(metrics.estimated_time_remaining);
          }
          if (metrics.total_tokens !== undefined) {
            setTotalTokens(metrics.total_tokens);
          }
        },
      })
      .catch(() => {});

    return () => {
      unlistenMetrics?.then((unlisten: any) => unlisten?.());
    };
  }, []);

  React.useEffect(() => {
    const unlistenDiagnostics = (window as any)
      .__TAURI_INVOKE__?.("listen", {
        event: "agent:diagnostics",
        handler: (event: any) => {
          const diagnostics = event.payload;
          if (diagnostics?.type === "prompt_truncation") {
            setPromptDiagnostics(diagnostics);
          }
        },
      })
      .catch(() => {});

    return () => {
      unlistenDiagnostics?.then((unlisten: any) => unlisten?.());
    };
  }, []);

  // Event listener for task clarification
  React.useEffect(() => {
    const unlistenClarification = (window as any)
      .__TAURI_INVOKE__?.("listen", {
        event: "agent:clarification",
        handler: (event: any) => {
          setClarificationData(event.payload);
        },
      })
      .catch(() => {});

    return () => {
      unlistenClarification?.then((unlisten: any) => unlisten?.());
    };
  }, []);

  // Event listener for loop recovery
  React.useEffect(() => {
    const unlistenLoopRecovery = (window as any)
      .__TAURI_INVOKE__?.("listen", {
        event: "agent:loop_recovery",
        handler: (event: any) => {
          setLoopRecoveryData(event.payload);
        },
      })
      .catch(() => {});

    return () => {
      unlistenLoopRecovery?.then((unlisten: any) => unlisten?.());
    };
  }, []);

  // Event listener for confidence scoring
  React.useEffect(() => {
    const unlistenConfidence = (window as any)
      .__TAURI_INVOKE__?.("listen", {
        event: "agent:confidence",
        handler: (event: any) => {
          setConfidenceData(event.payload);
        },
      })
      .catch(() => {});

    return () => {
      unlistenConfidence?.then((unlisten: any) => unlisten?.());
    };
  }, []);

  // Event listener for reasoning
  React.useEffect(() => {
    const unlistenReasoning = (window as any)
      .__TAURI_INVOKE__?.("listen", {
        event: "agent:reasoning",
        handler: (event: any) => {
          setReasoningData(event.payload);
        },
      })
      .catch(() => {});

    return () => {
      unlistenReasoning?.then((unlisten: any) => unlisten?.());
    };
  }, []);

  // Event listener for context integration
  React.useEffect(() => {
    const unlistenContextIntegration = (window as any)
      .__TAURI_INVOKE__?.("listen", {
        event: "agent:context_integration",
        handler: (event: any) => {
          setContextIntegrationData(event.payload);
        },
      })
      .catch(() => {});

    return () => {
      unlistenContextIntegration?.then((unlisten: any) => unlisten?.());
    };
  }, []);

  React.useEffect(() => {
    if (!isLoading) {
      setRespondedSteps({});
      setCountdown(null);
      // Reset metrics when loading completes
      setTokensPerSecond(undefined);
      setEstimatedTimeRemaining(undefined);
      setTotalTokens(undefined);
      setPhaseHistory([]);
      setPromptDiagnostics(null);
      // Reset Kiro behavior state
      setClarificationData(null);
      setLoopRecoveryData(null);
      setConfidenceData(null);
      setReasoningData(null);
      setContextIntegrationData(null);
      // Reset alwaysRun on task completion? Or keep it? Usually better to reset for safety.
      // setAlwaysRun(false);
    } else {
      // Reset phase start time when loading begins
      setPhaseStartTime(Date.now());
      setPhaseHistory([]);
    }
  }, [isLoading]);

  // Update elapsed time every second — use a real counter instead of a
  // no-op setState trick which forces unnecessary re-renders.
  React.useEffect(() => {
    if (!isLoading) {
      setElapsedSeconds(0);
      return;
    }
    setElapsedSeconds(0);
    const timer = setInterval(() => {
      setElapsedSeconds((s) => s + 1);
    }, 1000);
    return () => clearInterval(timer);
  }, [isLoading, phaseStartTime]);

  const onPermissionClick = (approved: boolean, idx: number) => {
    setRespondedSteps((prev) => ({ ...prev, [idx]: true }));
    setCountdown(null);
    handlePermissionResponse(approved, idx);
  };

  const imageInputRef = React.useRef<HTMLInputElement>(null);
  const handleImageChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!e.target.files) return;
    Array.from(e.target.files).forEach((file) => {
      const reader = new FileReader();
      reader.onloadend = () => {
        const base64 = reader.result as string;
        setSelectedImages((prev) => [...prev, base64]);
      };
      reader.readAsDataURL(file);
    });
    e.target.value = ""; // Reset for next selection
  };

  const pendingPermissionStepIdx = agentSteps.findIndex(
    (s, i) => s.status === "awaiting_permission" && !respondedSteps[i],
  );
  const pendingPermissionStep =
    pendingPermissionStepIdx >= 0 ? agentSteps[pendingPermissionStepIdx] : null;
  const isHighRiskPermission = pendingPermissionStep
    ? isHighRiskPermissionSummary(pendingPermissionStep.summary)
    : false;

  // Handle auto-run logic
  React.useEffect(() => {
    if (
      alwaysRun &&
      !isHighRiskPermission &&
      pendingPermissionStepIdx >= 0 &&
      !respondedSteps[pendingPermissionStepIdx] &&
      countdown === null
    ) {
      setCountdown(3); // 3 second countdown
    } else if (
      !alwaysRun ||
      isHighRiskPermission ||
      pendingPermissionStepIdx < 0
    ) {
      setCountdown(null);
    }
  }, [
    alwaysRun,
    isHighRiskPermission,
    pendingPermissionStepIdx,
    respondedSteps,
  ]);

  React.useEffect(() => {
    if (countdown !== null && countdown > 0) {
      const timer = setTimeout(() => setCountdown(countdown - 1), 1000);
      return () => clearTimeout(timer);
    } else if (countdown === 0 && pendingPermissionStepIdx >= 0) {
      onPermissionClick(true, pendingPermissionStepIdx);
    }
  }, [countdown, pendingPermissionStepIdx]);

  const getCurrentThought = (content: string) => {
    if (!content) return null;
    const sanitizeThought = (value: string) =>
      value
        .replace(/\\"/g, '"')
        .replace(/\\\\/g, "\\")
        .replace(/",?\s*"tool"\s*:\s*[^]*$/, "")
        .replace(/\}\s*$/, "")
        .replace(/^[{\s"]+/, "")
        .replace(/^thought\b[:\s-]*/i, "")
        .trim();

    // Extract thought from JSON format: {"thought": "...", ...}
    const completeThoughtRegex = /"thought"\s*:\s*"([^"\\]*(?:\\.[^"\\]*)*)"/;
    const partialThoughtRegex = /"thought"\s*:\s*"([^]*)$/;
    const match =
      content.match(completeThoughtRegex) || content.match(partialThoughtRegex);

    if (match && match[1]) {
      const thought = sanitizeThought(match[1]);
      return thought.length > 150 ? "..." + thought.slice(-150) : thought;
    }

    const fallback = sanitizeThought(content);
    if (fallback && fallback !== content.trim()) {
      return fallback.length > 150 ? "..." + fallback.slice(-150) : fallback;
    }

    return null;
  };

  const activeThought = getCurrentThought(liveStreamingContent);

  if (!isChatOpen) return null;

  return (
    <>
      <div className="chat-resize-handle" onMouseDown={handleChatResize} />
      <div className="chat-panel glass" style={{ width: `${chatWidth}px` }}>
        <div className="chat-panel-header">
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <svg
              width="15"
              height="15"
              viewBox="0 0 24 24"
              fill="none"
              stroke="var(--accent-primary)"
              strokeWidth="2"
            >
              <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
            </svg>
            <span style={{ fontWeight: 600, fontSize: 12 }}>
              WHIZCODE AGENT
            </span>
          </div>
          <div style={{ display: "flex", gap: 4 }}>
            <div
              className="chat-header-btn"
              onClick={handleReset}
              title="Reset conversation"
            >
              <svg
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
              >
                <path d="M23 4v6h-6"></path>
                <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
              </svg>
            </div>
            <div
              className="chat-header-btn"
              onClick={() => setIsChatOpen(false)}
              title="Close panel"
            >
              ×
            </div>
          </div>
        </div>

        {workspacePath && (
          <div className="chat-context-bar">
            <svg
              width="12"
              height="12"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
            </svg>
            <span>{workspacePath.split(/[/\\]/).pop()}</span>
            <span className="context-connected">● Context loaded</span>
          </div>
        )}

        <ChatSettings {...settingsProps} />
        <SpecPlanPanel
          currentPlan={currentPlan}
          activeSpec={activeSpec}
          taskSnapshot={taskSnapshot}
          isLoading={isLoading}
        />

        <div className="chat-messages">
          <ArchivedMessagesList messages={messages} getToolIcon={getToolIcon} />

          {isLoading && (
            <div className="chat-msg assistant">
              <div className="chat-msg-sender">WHIZCODE</div>
              <div className="chat-msg-content">
                <LiveAgentActivity
                  liveStreamingContent={liveStreamingContent}
                  agentSteps={agentSteps}
                  getToolIcon={getToolIcon}
                />
              </div>
            </div>
          )}

          {clarificationData && (
            <div className="chat-msg assistant">
              <div className="chat-msg-sender">WHIZCODE</div>
              <div className="chat-msg-content">
                <TaskClarificationPanel
                  clarification={clarificationData}
                  onApprove={() => {}}
                  onModify={() => {}}
                  onCancel={() => {}}
                />
              </div>
            </div>
          )}

          {loopRecoveryData && (
            <div className="chat-msg assistant">
              <div className="chat-msg-sender">WHIZCODE</div>
              <div className="chat-msg-content">
                <LoopRecoveryPanel guidance={loopRecoveryData} />
              </div>
            </div>
          )}

          {confidenceData && (
            <div className="chat-msg assistant">
              <div className="chat-msg-sender">WHIZCODE</div>
              <div className="chat-msg-content">
                <ConfidencePanel confidence={confidenceData} />
              </div>
            </div>
          )}

          {reasoningData && (
            <div className="chat-msg assistant">
              <div className="chat-msg-sender">WHIZCODE</div>
              <div className="chat-msg-content">
                <ReasoningPanel reasoning={reasoningData} />
              </div>
            </div>
          )}

          {contextIntegrationData && (
            <div className="chat-msg assistant">
              <div className="chat-msg-sender">WHIZCODE</div>
              <div className="chat-msg-content">
                <ContextIntegrationPanel context={contextIntegrationData} />
              </div>
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>

        <div className="chat-input-area">
          {isLoading && (
            <LiveThoughtPanel
              currentPhase={currentPhase}
              elapsedSeconds={elapsedSeconds}
              liveStreamingContent={liveStreamingContent}
              activeThought={activeThought}
              agentSteps={agentSteps}
              tokensPerSecond={tokensPerSecond}
              estimatedTimeRemaining={estimatedTimeRemaining}
              totalTokens={totalTokens}
              promptDiagnostics={promptDiagnostics}
            />
          )}
          {pendingPermissionStepIdx >= 0 && (
            <div
              className="permission-controls-enhanced"
              style={{
                display: "flex",
                flexDirection: "column",
                gap: "10px",
                padding: "12px",
                background: "var(--vscode-bg)",
                border: "1px solid var(--accent-primary)",
                borderRadius: "6px",
                marginBottom: "10px",
                boxShadow: "0 -4px 12px rgba(0,0,0,0.3)",
                borderLeft: "4px solid var(--accent-primary)",
              }}
            >
              <div
                style={{
                  display: "flex",
                  alignItems: "flex-start",
                  gap: "10px",
                }}
              >
                <div style={{ fontSize: "18px", marginTop: "2px" }}>🛡️</div>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div
                    style={{
                      fontSize: "11px",
                      color: "var(--text-secondary)",
                      fontWeight: 600,
                      textTransform: "uppercase",
                      marginBottom: "4px",
                    }}
                  >
                    Permission Required
                  </div>
                  <div
                    style={{
                      fontSize: "13px",
                      fontWeight: 400,
                      lineHeight: "1.4",
                      wordBreak: "break-word",
                      overflowWrap: "anywhere",
                      color: "var(--text-primary)",
                      fontFamily: "var(--font-mono)",
                      background: "rgba(0,0,0,0.2)",
                      padding: "6px",
                      borderRadius: "4px",
                    }}
                  >
                    {agentSteps[pendingPermissionStepIdx].summary}
                  </div>
                  {isHighRiskPermission && (
                    <div
                      style={{
                        marginTop: "8px",
                        fontSize: "11px",
                        color: "#f9e2af",
                        background: "rgba(249, 226, 175, 0.08)",
                        border: "1px solid rgba(249, 226, 175, 0.2)",
                        padding: "6px",
                        borderRadius: "4px",
                      }}
                    >
                      High-risk command detected. Auto-run is disabled for this
                      step.
                    </div>
                  )}
                </div>
              </div>

              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  borderTop: "1px solid var(--border-color)",
                  paddingTop: "8px",
                }}
              >
                <label
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "6px",
                    cursor: "pointer",
                    userSelect: "none",
                  }}
                >
                  <input
                    type="checkbox"
                    checked={alwaysRun}
                    onChange={(e) => setAlwaysRun(e.target.checked)}
                    disabled={isHighRiskPermission}
                    style={{ accentColor: "var(--accent-primary)" }}
                  />
                  <span
                    style={{ fontSize: "11px", color: "var(--text-secondary)" }}
                  >
                    {isHighRiskPermission
                      ? "Always run disabled for high-risk commands"
                      : "Always run in this interaction"}
                  </span>
                </label>

                <div style={{ display: "flex", gap: "8px" }}>
                  <button
                    className="perm-btn deny"
                    onClick={() =>
                      onPermissionClick(false, pendingPermissionStepIdx)
                    }
                    disabled={
                      respondedSteps[pendingPermissionStepIdx] || !isLoading
                    }
                    style={{ padding: "4px 12px" }}
                  >
                    Deny
                  </button>
                  <button
                    className="perm-btn approve"
                    onClick={() =>
                      onPermissionClick(true, pendingPermissionStepIdx)
                    }
                    disabled={
                      respondedSteps[pendingPermissionStepIdx] || !isLoading
                    }
                    style={{
                      padding: "4px 20px",
                      minWidth: "80px",
                      position: "relative",
                    }}
                  >
                    {countdown !== null ? `Run (${countdown}s)` : "Run"}
                  </button>
                </div>
              </div>
            </div>
          )}
          {selectedImages.length > 0 && (
            <div
              className="image-previews"
              style={{
                display: "flex",
                gap: "8px",
                marginBottom: "8px",
                overflowX: "auto",
                padding: "4px",
              }}
            >
              {selectedImages.map((img, i) => (
                <div
                  key={i}
                  style={{
                    position: "relative",
                    width: "60px",
                    height: "60px",
                    flexShrink: 0,
                  }}
                >
                  <img
                    src={img}
                    style={{
                      width: "100%",
                      height: "100%",
                      objectFit: "cover",
                      borderRadius: "4px",
                      border: "1px solid var(--border-color)",
                    }}
                  />
                  <button
                    onClick={() =>
                      setSelectedImages((prev) =>
                        prev.filter((_, idx) => idx !== i),
                      )
                    }
                    style={{
                      position: "absolute",
                      top: "-6px",
                      right: "-6px",
                      background: "#e74c3c",
                      color: "white",
                      border: "none",
                      borderRadius: "50%",
                      width: "16px",
                      height: "16px",
                      fontSize: "10px",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      cursor: "pointer",
                    }}
                  >
                    ×
                  </button>
                </div>
              ))}
            </div>
          )}
          <div className="chat-input-box">
            <input
              type="file"
              ref={imageInputRef}
              style={{ display: "none" }}
              accept="image/*"
              multiple
              onChange={handleImageChange}
            />
            <button
              onClick={() => imageInputRef.current?.click()}
              disabled={isLoading}
              style={{
                background: "transparent",
                border: "none",
                fontSize: "18px",
                cursor: "pointer",
                padding: "0 8px",
                opacity: isLoading ? 0.3 : 0.7,
                transition: "opacity 0.2s",
              }}
              onMouseEnter={(e) => (e.currentTarget.style.opacity = "1")}
              onMouseLeave={(e) => (e.currentTarget.style.opacity = "0.7")}
            >
              🖼️
            </button>
            <textarea
              className="chat-input"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              onPaste={(e) => {
                const items = e.clipboardData.items;
                for (let i = 0; i < items.length; i++) {
                  if (items[i].type.indexOf("image") !== -1) {
                    const file = items[i].getAsFile();
                    if (file) {
                      const reader = new FileReader();
                      reader.onloadend = () => {
                        const base64 = reader.result as string;
                        setSelectedImages((prev) => [...prev, base64]);
                      };
                      reader.readAsDataURL(file);
                    }
                  }
                }
              }}
              placeholder={
                workspacePath
                  ? "Ask about your code..."
                  : "Open a folder first..."
              }
              rows={3}
              disabled={isLoading}
            />
            {!isLoading ? (
              <button
                className="send-btn"
                onClick={() => handleSend()}
                disabled={!input.trim() && selectedImages.length === 0}
              >
                <svg className="send-icon" viewBox="0 0 24 24">
                  <path
                    d="M22 2L11 13"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                  <path
                    d="M22 2L15 22L11 13L2 9L22 2Z"
                    fill="currentColor"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                </svg>
              </button>
            ) : (
              <button
                className="stop-btn"
                onClick={() => handleStop()}
                title="Stop Agent"
              >
                <svg
                  className="stop-icon"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                >
                  <rect
                    x="3"
                    y="3"
                    width="18"
                    height="18"
                    rx="2"
                    ry="2"
                    fill="currentColor"
                  ></rect>
                </svg>
              </button>
            )}
          </div>
        </div>
      </div>
    </>
  );
};
