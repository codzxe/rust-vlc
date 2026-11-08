// * (not-found) — the Solstice v2 404 (2026-11-09): a STAGE screen, not a
// spool gag. While mounted it flips the app backdrop to full-strength
// sunset (`html[data-backdrop="vivid"]`, removed on unmount — the same
// mount-effect pattern as onboarding), and sets one theatrical composition:
// the sun is slipping below the horizon — a warm disc with its lower ~40%
// already swallowed by the ground band, its afterglow bleeding along the
// horizon line — with a lost
// sticky note floating at the optical center of the stage. The note carries
// a big handwritten-style "404" page number (decorative, aria-hidden — the
// accessible name stays with the real heading), a clear title, and exactly
// ONE way back to the player.
//
// The screen renders NO <main> of its own — the AppShell provides it.

import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "../../components/button";
import { useMsg } from "../../i18n/messages";
import "./not-found.css";

export function NotFoundScreen() {
  const m = useMsg();
  const navigate = useNavigate();

  // Stage tier (brief §1): the 404 is an emotional moment — full-strength
  // sunset for exactly as long as this screen is mounted, never longer.
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.backdrop = "vivid";
    return () => {
      delete root.dataset.backdrop;
    };
  }, []);

  return (
    <div className="not-found solstice-rise">
      {/* Scene, painted strictly BEHIND the card (z 0/1 vs the note's z 2):
       * the sun sits mid-set — its center rides 18px ABOVE the horizon
       * line (band top at 30% of the stage) with the lower ~40% of the disc
       * already under the world's edge, so it reads as slipping below it,
       * never as an ornament fighting the card (craft rule §2.2). The
       * afterglow rim glows along the horizon; the torn tape fragment
       * floating in the sky is where the lost note fell from. */}
      <span className="not-found__tape" aria-hidden="true" />
      <span className="not-found__sun solstice-sun" aria-hidden="true" />
      <span className="not-found__horizon" aria-hidden="true" />
      <section
        className="not-found__card solstice-note"
        data-tape="left"
        aria-labelledby="not-found-title"
      >
        {/* The page number, written large on the lost note like a bookmark
         * scribble — decorative emphasis of the route status; the heading
         * below carries the accessible title. */}
        <span className="not-found__code" aria-hidden="true">
          404
        </span>
        <h1 id="not-found-title" className="not-found__title">
          {m("not-found.title")}
        </h1>
        <p className="not-found__body">{m("not-found.body")}</p>
        <Button
          variant="primary"
          size="lg"
          className="not-found__cta"
          onClick={() => navigate("/player")}
        >
          {m("not-found.action")}
        </Button>
      </section>
    </div>
  );
}
