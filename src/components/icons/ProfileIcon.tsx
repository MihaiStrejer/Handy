import { useState } from "react";
import {
  Mail,
  NotebookPen,
  SquareCode,
  Terminal,
  UserRound,
} from "lucide-react";
import chromeIcon from "./profile-assets/chrome.svg";
import firefoxIcon from "./profile-assets/firefox.svg";
import vscodeIcon from "./profile-assets/vscode.ico?url";
import intellijIcon from "./profile-assets/intellij.svg";
import claudeIcon from "./profile-assets/claude.svg";
import codexIcon from "./profile-assets/codex.png";
import piIcon from "./profile-assets/pi.svg";
import opencodeIcon from "./profile-assets/opencode.svg";
import githubIcon from "./profile-assets/github.svg";
import slackIcon from "./profile-assets/slack.ico?url";

export const BUILTIN_PROFILE_ICONS = [
  "generic",
  "terminal",
  "code",
  "mail",
  "chrome",
  "firefox",
  "vscode",
  "intellij",
  "claude",
  "codex",
  "pi",
  "opencode",
  "github",
  "slack",
  "notes",
] as const;

export type BuiltinProfileIcon = (typeof BUILTIN_PROFILE_ICONS)[number];

const brandIcons: Partial<Record<BuiltinProfileIcon, string>> = {
  chrome: chromeIcon,
  firefox: firefoxIcon,
  vscode: vscodeIcon,
  intellij: intellijIcon,
  claude: claudeIcon,
  codex: codexIcon,
  pi: piIcon,
  opencode: opencodeIcon,
  github: githubIcon,
  slack: slackIcon,
};

const monochromeBrands = new Set<BuiltinProfileIcon>([
  "chrome",
  "firefox",
  "intellij",
  "claude",
  "opencode",
  "github",
]);

const lineIcons = {
  generic: UserRound,
  terminal: Terminal,
  code: SquareCode,
  mail: Mail,
  notes: NotebookPen,
};

type ProfileIconProps = {
  icon: string | { custom: string };
  size?: number;
  className?: string;
};

export function ProfileIcon({ icon, size = 20, className }: ProfileIconProps) {
  const [failedSource, setFailedSource] = useState<string | null>(null);
  const customSource = typeof icon === "object" ? icon.custom : null;
  const source = customSource?.startsWith("data:image/png;base64,")
    ? customSource
    : typeof icon === "string" && icon.startsWith("data:image/png;base64,")
      ? icon
      : typeof icon === "string"
        ? brandIcons[icon as BuiltinProfileIcon]
        : undefined;

  if (
    source &&
    typeof icon === "string" &&
    monochromeBrands.has(icon as BuiltinProfileIcon)
  ) {
    return (
      <span
        className={className}
        style={{
          display: "inline-block",
          width: size,
          height: size,
          flexShrink: 0,
          backgroundColor: "currentColor",
          mask: `url("${source}") center / contain no-repeat`,
        }}
        aria-hidden="true"
      />
    );
  }

  if (source && failedSource !== source) {
    return (
      <img
        src={source}
        width={size}
        height={size}
        className={className}
        style={{ objectFit: "contain", flexShrink: 0 }}
        alt=""
        aria-hidden="true"
        onError={() => setFailedSource(source)}
      />
    );
  }

  const Icon =
    typeof icon === "string" && icon in lineIcons
      ? lineIcons[icon as keyof typeof lineIcons]
      : UserRound;
  return <Icon size={size} className={className} aria-hidden="true" />;
}
