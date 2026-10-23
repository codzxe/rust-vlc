import { useMsg, type MessageKey } from "../i18n/messages";

// Honest placeholder for a doc 08 §1.6 design screen: renders the route's
// name and an explicit "not implemented yet" body. No design is faked here.
export function ScreenPlaceholder({
  routeKey,
  detail,
}: {
  routeKey: MessageKey;
  detail?: string;
}) {
  const m = useMsg();
  return (
    <main data-screen={routeKey} className="screen-placeholder">
      <h1>{m(routeKey)}</h1>
      <p>{m("screen.placeholder.body")}</p>
      {detail === undefined ? null : (
        <p>
          <code>{detail}</code>
        </p>
      )}
    </main>
  );
}
