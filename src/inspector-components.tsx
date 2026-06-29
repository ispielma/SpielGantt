import {
  Alert,
  Badge,
  Code,
  Fieldset,
  Group,
  Paper,
  Stack,
  Text,
  Title,
  type MantineSpacing,
} from "@mantine/core";
import type { ReactNode, Ref } from "react";

export function InspectorSurface(props: {
  "aria-label": string;
  children: ReactNode;
  className?: string;
  contentClassName?: string;
  contentLayout?: string;
  contentTestId?: string;
  surfaceRef?: Ref<HTMLElement>;
  testId?: string;
}) {
  const className = ["inspector-surface", props.className].filter(Boolean).join(" ");
  return (
    <Paper
      aria-label={props["aria-label"]}
      className={className}
      component="section"
      data-testid={props.testId}
      p="lg"
      radius="md"
      ref={props.surfaceRef}
      withBorder
    >
      <Stack
        className={props.contentClassName}
        data-layout={props.contentLayout}
        data-testid={props.contentTestId}
        gap="md"
      >
        {props.children}
      </Stack>
    </Paper>
  );
}

export function EmptyWorkspacePanel(props: {
  eyebrow: string;
  title: string;
  className?: string;
  testId?: string;
}) {
  const className = ["project-empty-state", props.className].filter(Boolean).join(" ");
  return (
    <Paper className={className} data-testid={props.testId} p="xl" radius="md" withBorder>
      <Stack gap="xs">
        <Text c="dimmed" fw={650} size="xs">
          {props.eyebrow}
        </Text>
        <Title order={2} size="h3">
          {props.title}
        </Title>
      </Stack>
    </Paper>
  );
}

export function InspectorSection(props: {
  children: ReactNode;
  legend: string;
  gap?: MantineSpacing;
  testId?: string;
}) {
  return (
    <Fieldset
      aria-label={props.legend}
      className="inspector-section"
      data-testid={props.testId}
      legend={props.legend}
      radius="md"
      role="group"
    >
      <Stack gap={props.gap ?? "sm"}>{props.children}</Stack>
    </Fieldset>
  );
}

export function InspectorToken(props: { children: ReactNode; tone?: "default" | "accent" }) {
  if (props.tone === "accent") {
    return (
      <Badge className="inspector-token" radius="xl" variant="light">
        {props.children}
      </Badge>
    );
  }

  return (
    <Code className="inspector-token" component="span">
      {props.children}
    </Code>
  );
}

export function InspectorTokenList(props: {
  "aria-label"?: string;
  emptyMessage?: string;
  items: string[];
  tone?: "default" | "accent";
}) {
  if (props.items.length === 0) {
    return (
      <Text c="dimmed" size="sm">
        {props.emptyMessage ?? "None"}
      </Text>
    );
  }

  return (
    <Group aria-label={props["aria-label"]} component="ul" gap="xs" m={0} p={0}>
      {props.items.map((item) => (
        <li className="inspector-token-list-item" key={item}>
          <InspectorToken tone={props.tone}>{item}</InspectorToken>
        </li>
      ))}
    </Group>
  );
}

export function TaskOperationAlert(props: {
  children: ReactNode;
  testId?: string;
}) {
  return (
    <Alert
      aria-live="polite"
      className="task-operation-status"
      color="yellow"
      data-status="error"
      data-testid={props.testId}
      radius="md"
      variant="light"
    >
      {props.children}
    </Alert>
  );
}
