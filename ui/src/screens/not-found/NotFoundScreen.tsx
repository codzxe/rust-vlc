// * (not-found) — the Solstice 404: sun ornament, sticky-note card, and a
// way back to the player (Solstice app-shell workstream, 2026-11-08).
//
// The screen renders NO <main> of its own — the AppShell provides it.

import { useNavigate } from "react-router-dom";
import { Button } from "../../components/button/Button";
import { useMsg } from "../../i18n/messages";
import "./not-found.css";

export function NotFoundScreen() {
  const m = useMsg();
  const navigate = useNavigate();

  return (
    <div className="not-found solstice-rise">
      <span className="not-found__sun solstice-sun" aria-hidden="true" />
      <section className="not-found__card solstice-note" data-tape="left" aria-labelledby="not-found-title">
        <h1 id="not-found-title" className="not-found__title">
          {m("not-found.title")}
        </h1>
        <p className="not-found__body">{m("not-found.body")}</p>
        <Button variant="primary" onClick={() => navigate("/player")}>
          {m("not-found.action")}
        </Button>
      </section>
    </div>
  );
}
