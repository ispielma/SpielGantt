import type { CSSProperties } from "react";
import type { TimelineViewProps } from "./timeline-panel-types.ts";
import {
  describeTaskAxisSpan,
  eventRailGridColumn,
  shouldStaggerEventLabels,
} from "./timeline-panel-view-model.ts";

export type {
  TimelinePlacedTaskView,
  TimelineStatusTaskView,
  TimelineViewProps,
} from "./timeline-panel-types.ts";

export function TimelineView(props: TimelineViewProps) {
  const {
    hasEvents,
    events,
    eventRailColumns,
    placedTasks,
    selectedTaskId = null,
    selectedEventId = null,
  } = props;
  const staggerEventLabels = shouldStaggerEventLabels(events);
  const eventRailGridRow = `1 / ${placedTasks.length + 2}`;

  if (!hasEvents) {
    return (
      <section className="timeline-view" data-testid="timeline-view" aria-label="Timeline">
        <div className="timeline-header">
          <div>
            <p className="eyebrow">Timeline</p>
            <h2>No project events yet.</h2>
          </div>
        </div>
      </section>
    );
  }

  return (
    <section className="timeline-view" data-testid="timeline-view" aria-label="Timeline">
      <div className="timeline-header">
        <div>
          <p className="eyebrow">Timeline</p>
        </div>
      </div>
      <div className="timeline-scroll">
        <div
          className="timeline-grid timeline-grid--events"
          style={{ gridTemplateColumns: eventRailColumns }}
        >
          <div aria-hidden="true" className="timeline-axis-spacer" />
          <div
            aria-hidden="true"
            className="timeline-event-guide-layer"
            style={{ gridTemplateColumns: eventRailColumns }}
          >
            {events.map((eventId, index) => {
              const isLastEvent = index === events.length - 1;
              const labelRow = staggerEventLabels ? (index % 2) + 1 : 1;
              return (
                <span
                  key={eventId}
                  className="timeline-event-rail-guide"
                  data-event-rail-end={isLastEvent ? "true" : undefined}
                  data-label-row={labelRow}
                  data-testid="timeline-event-rail-guide"
                  style={{
                    gridColumn: eventRailGridColumn(index, events.length),
                  }}
                />
              );
            })}
          </div>
          {events.map((eventId, index) => {
            const isLastEvent = index === events.length - 1;
            const labelRow = staggerEventLabels ? (index % 2) + 1 : 1;
            return (
              <div
                key={eventId}
                aria-label={`Event rail ${eventId}`}
                className={`timeline-event-rail${
                  isLastEvent ? " timeline-event-rail--end" : ""
                }`}
                data-event-id={eventId}
                data-label-row={labelRow}
                data-testid="timeline-event-rail"
                style={{
                  gridColumn: eventRailGridColumn(index, events.length),
                  gridRow: eventRailGridRow,
                }}
              >
                <button
                  type="button"
                  aria-label={`Select event ${eventId}`}
                  aria-pressed={selectedEventId === eventId}
                  className="timeline-event-rail-button"
                  data-label-row={labelRow}
                  data-event-id={eventId}
                  data-timeline-event-select="true"
                >
                  <span
                    aria-label={eventId}
                    className="timeline-event-label"
                    data-label-row={labelRow}
                    title={eventId}
                  >
                    {eventId}
                  </span>
                </button>
              </div>
            );
          })}
          {placedTasks.map((task) => (
            <div key={task.taskId} className="timeline-row">
              <div
                className="timeline-track"
                data-column-count={Math.max(events.length - 1, 1)}
                data-task-id={task.taskId}
                data-testid="timeline-track"
                style={
                  {
                    gridColumn: "1 / -1",
                    gridTemplateColumns: eventRailColumns,
                  } as CSSProperties
                }
              >
                <button
                  type="button"
                  aria-label={describeTaskAxisSpan(task)}
                  aria-pressed={selectedTaskId === task.taskId}
                  className="timeline-bar timeline-bar--event-axis"
                  data-drag-enabled="false"
                  data-event-dependency-ids={
                    task.dependencyIds.length > 0 ? task.dependencyIds.join(",") : undefined
                  }
                  data-event-span-end-id={task.endEventId ?? undefined}
                  data-event-span-start-id={task.startEventId ?? undefined}
                  data-layout-state={task.layoutState}
                  data-has-workflow-warning={
                    task.warningDiagnostics?.length ? "true" : undefined
                  }
                  data-task-status={task.status ?? "unblocked"}
                  data-task-id={task.taskId}
                  data-testid="timeline-bar"
                  data-timeline-task-select="true"
                  style={{
                    gridColumn: task.gridColumn,
                    marginLeft:
                      task.inlineStartPercent > 0 ? `${task.inlineStartPercent}%` : undefined,
                    width:
                      task.inlineSizePercent < 100 ? `${task.inlineSizePercent}%` : undefined,
                  }}
                >
                  {task.warningDiagnostics?.length ? (
                    <span
                      aria-hidden="true"
                      className="timeline-bar-warning"
                      data-testid="timeline-warning-indicator"
                      title={task.warningDiagnostics.join(" ")}
                    >
                      !
                    </span>
                  ) : null}
                  <span className="timeline-bar-label">{task.taskId}</span>
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
