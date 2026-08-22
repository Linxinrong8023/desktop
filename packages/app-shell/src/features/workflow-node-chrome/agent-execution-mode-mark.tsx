import { IconUserSearch } from "@tabler/icons-react";
import { useTranslation } from "react-i18next";
import { cn } from "@ora/ui";

interface AgentExecutionModeMarkProps {
  interactive: boolean;
  className?: string;
}

/** Identifies whether an Agent node pauses for human conversation or runs automatically. */
export function AgentExecutionModeMark({
  interactive,
  className,
}: AgentExecutionModeMarkProps) {
  const { t } = useTranslation();
  const mode = interactive ? "interactive" : "automatic";
  const label = t(`workflowNode.agentExecutionMode.${mode}`);

  return (
    <span
      role="img"
      aria-label={label}
      title={label}
      data-agent-execution-mode={mode}
      className={cn(
        "inline-flex size-4 shrink-0 items-center justify-center text-muted-foreground",
        className,
      )}
    >
      {interactive ? (
        <IconUserSearch className="size-4" stroke={1.9} aria-hidden />
      ) : (
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.9"
          strokeLinecap="round"
          strokeLinejoin="round"
          className="size-4"
          aria-hidden="true"
        >
          <rect x="2" y="3" width="20" height="14" rx="2" />
          <path d="m10 8 5 3-5 3Z" />
          <path d="M12 17v4" />
          <path d="M8 21h8" />
        </svg>
      )}
    </span>
  );
}
