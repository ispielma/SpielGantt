export type AccessibleRole =
  | "alert"
  | "button"
  | "combobox"
  | "dialog"
  | "group"
  | "link"
  | "menu"
  | "menuitem"
  | "option"
  | "textbox"
  | "tree"
  | "treeitem";

export function accessibleName(element: Element): string {
  const ariaLabel = element.getAttribute("aria-label");
  if (ariaLabel) {
    return ariaLabel.trim();
  }

  const id = element.getAttribute("id");
  if (id) {
    const ownerDocument = element.ownerDocument;
    const label = Array.from(ownerDocument.querySelectorAll("label")).find(
      (candidate) => candidate.getAttribute("for") === id,
    );
    if (label?.textContent) {
      return label.textContent.trim();
    }
  }

  return (element.textContent ?? "").replace(/\s+/g, " ").trim();
}

export function elementRole(element: Element): AccessibleRole | null {
  const explicitRole = element.getAttribute("role");
  if (
    explicitRole === "button" ||
    explicitRole === "alert" ||
    explicitRole === "combobox" ||
    explicitRole === "dialog" ||
    explicitRole === "group" ||
    explicitRole === "link" ||
    explicitRole === "menu" ||
    explicitRole === "menuitem" ||
    explicitRole === "option" ||
    explicitRole === "textbox" ||
    explicitRole === "tree" ||
    explicitRole === "treeitem"
  ) {
    return explicitRole;
  }

  const tagName = element.tagName.toLowerCase();
  if (tagName === "button") {
    return "button";
  }
  if (tagName === "a" && element.hasAttribute("href")) {
    return "link";
  }
  if (tagName === "select") {
    return "combobox";
  }
  if (tagName === "option") {
    return "option";
  }
  if (tagName === "textarea") {
    return "textbox";
  }
  if (tagName === "input") {
    return "textbox";
  }

  return null;
}

export function getByRole(root: Element, role: AccessibleRole, name: RegExp): HTMLElement {
  const match = Array.from(root.querySelectorAll("*")).find(
    (element) => elementRole(element) === role && name.test(accessibleName(element)),
  );
  if (!match) {
    throw new Error(`expected ${role} named ${name} to be present`);
  }
  return match as HTMLElement;
}

export function interactiveElements(root: Element): Element[] {
  return Array.from(root.querySelectorAll("*")).filter((element) => elementRole(element) !== null);
}
