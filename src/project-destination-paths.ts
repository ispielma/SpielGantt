function pathSeparatorFor(parentDestination: string): string {
  return parentDestination.includes("\\") && !parentDestination.includes("/") ? "\\" : "/";
}

export function joinProjectDestinationPath(
  parentDestination: string,
  projectName: string,
): string {
  const separator = pathSeparatorFor(parentDestination);
  const normalizedParent =
    parentDestination.endsWith("/") || parentDestination.endsWith("\\")
      ? parentDestination.slice(0, -1)
      : parentDestination;

  return `${normalizedParent}${separator}${projectName}`;
}

export function projectFolderPreview(
  parentDestination: string | null,
  projectName: string,
): string {
  if (!parentDestination) {
    return "Resolving destination...";
  }

  const trimmedProjectName = projectName.trim();
  if (!trimmedProjectName) {
    return "Enter a project name to preview the final folder path.";
  }

  return joinProjectDestinationPath(parentDestination, trimmedProjectName);
}
