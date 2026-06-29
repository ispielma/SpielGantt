import type { CSSProperties } from "react";
import {
  IconCalendarEvent,
  IconCalendarPlus,
  IconDots,
  IconExternalLink,
  IconFilePlus,
  IconFolder,
  IconFolderOpen,
  IconFolderPlus,
  IconFolderSearch,
  IconListCheck,
  IconPlus,
  IconRefresh,
  IconTrash,
} from "@tabler/icons-react";
import type { TablerIcon } from "@tabler/icons-react";

interface SidebarIconProps {
  className?: string;
  size?: number;
  stroke?: number;
  "data-project-tree-icon"?: string;
}

const tablerIconStyle: CSSProperties = { fill: "none" };

function SidebarIcon({
  icon: Icon,
  className = "projects-tree-svg-icon",
  size = 14,
  stroke = 2,
  ...props
}: SidebarIconProps & { icon: TablerIcon }) {
  return (
    <Icon
      aria-hidden="true"
      className={className}
      focusable={false}
      size={size}
      stroke={stroke}
      style={tablerIconStyle}
      {...props}
    />
  );
}

export function ProjectFolderIcon({ expanded }: { expanded: boolean }) {
  return (
    <SidebarIcon
      data-project-tree-icon="project-folder"
      icon={expanded ? IconFolderOpen : IconFolder}
    />
  );
}

export function TasksIcon() {
  return <SidebarIcon icon={IconListCheck} />;
}

export function EventsIcon() {
  return <SidebarIcon icon={IconCalendarEvent} />;
}

export function ProjectActionsToggleIcon() {
  return <SidebarIcon className="project-actions-toggle-icon" icon={IconDots} size={16} />;
}

export function SectionCreateIcon() {
  return <SidebarIcon className="project-tree-section-create-icon" icon={IconPlus} size={16} />;
}

export function ProjectsMenuToggleIcon() {
  return <SidebarIcon className="projects-menu-toggle-icon" icon={IconFolderPlus} size={18} />;
}

export function MenuOpenProjectIcon() {
  return <SidebarIcon icon={IconExternalLink} />;
}

export function MenuRefreshIcon() {
  return <SidebarIcon icon={IconRefresh} />;
}

export function MenuRevealInFinderIcon() {
  return <SidebarIcon icon={IconFolderSearch} />;
}

export function MenuCreateTaskIcon() {
  return <SidebarIcon icon={IconFilePlus} />;
}

export function MenuCreateEventIcon() {
  return <SidebarIcon icon={IconCalendarPlus} />;
}

export function MenuDeleteProjectIcon() {
  return <SidebarIcon icon={IconTrash} />;
}
