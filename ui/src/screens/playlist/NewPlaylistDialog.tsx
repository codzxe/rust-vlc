// w26.b09 — the create-playlist dialog (new playlist.svg, 1:1 anatomy).
//
// Frame grammar (713×589 rx14 #191919 window, scaled to the app's 560px
// dialog column): header "New Playlist" (#8F8F8F) + ✕ close · "Playlist
// details" section (#C6C6C6 label + #6B6B6B helper) with the title input
// ("e.g., Favorite Videos" placeholder) and the 110×110 thumbnail tile ·
// hairline separator · "Select files" section with the drop zone ("Drag
// files into the box or click here to select files.") · footer Cancel
// (#1B1B1B/#8F8F8B) + Save (#371F06/#D37918 — the `tinted` Button).
//
// HONESTY: Save creates a LOCAL saved playlist (playlistsStore — no engine
// command exists); the thumbnail tile is disabled-honest (no backend, the
// a10 create-row rule); file picks/drops record NAMES only (the media-rows
// browse view is the wave-b thumbnail/media lane's). The title input uses
// the shared form control grammar (`lumen-field__control`, form lane) — the
// sheet is imported here because no form COMPONENT renders on this page
// (Vite dedupes; same class precedent as network-stream/convert screens).
// Copy resolves through the lane seam (usePlaylistMsg) — the keys are
// registered into the catalog by the Lead post-wave (contract §4.5).

import { useEffect, useId, useRef, useState, type ChangeEvent, type DragEvent } from "react";
import { Button, IconButton } from "../../components/button";
import { Icon } from "../../components/icon";
import { Dialog, Separator } from "../../components/surface";
import "../../components/form/form.css";
import { createPlaylist } from "./playlistsStore";
import { usePlaylistMsg } from "./usePlaylistMsg";
import "./playlist.css";

export function NewPlaylistDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const m = usePlaylistMsg();
  const titleId = useId();
  const helperId = useId();
  const fileInputId = useId();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const titleInputRef = useRef<HTMLInputElement>(null);
  const [title, setTitle] = useState("");
  const [fileNames, setFileNames] = useState<readonly string[]>([]);
  const [dropOver, setDropOver] = useState(false);

  // w26.i10 (h16 dialog nit): initial focus lands on the TITLE INPUT — the
  // dialog's primary control (APG dialog pattern; the network-stream lane's
  // url-field precedent), not the shared trap's default first-focusable
  // (the header ✕). The dialog CONTENT mounts inside the child Dialog, so
  // dev StrictMode double-invokes the child's trap effect AFTER this
  // component's update effect — the refocus rides a macrotask so it lands
  // after every effect pass (first field wins, Enter types, not dismisses).
  useEffect(() => {
    if (!open) return;
    const timer = window.setTimeout(() => titleInputRef.current?.focus(), 0);
    return () => window.clearTimeout(timer);
  }, [open]);

  if (!open) return null;

  const close = () => {
    setTitle("");
    setFileNames([]);
    setDropOver(false);
    onClose();
  };

  const recordFiles = (files: FileList | null) => {
    if (files === null || files.length === 0) return;
    setFileNames((prev) => {
      const seen = new Set(prev);
      const added: string[] = [];
      for (const file of Array.from(files)) {
        if (!seen.has(file.name)) {
          seen.add(file.name);
          added.push(file.name);
        }
      }
      return added.length > 0 ? [...prev, ...added] : prev;
    });
  };

  const onFileChange = (event: ChangeEvent<HTMLInputElement>) => {
    recordFiles(event.target.files);
    event.target.value = ""; // same file re-picked stays a change event
  };

  const onDrop = (event: DragEvent<HTMLElement>) => {
    event.preventDefault();
    setDropOver(false);
    recordFiles(event.dataTransfer.files);
  };

  const save = () => {
    const trimmed = title.trim();
    if (trimmed === "") return; // Save is disabled until a title exists
    createPlaylist({ title: trimmed, itemNames: fileNames });
    close();
  };

  return (
    <Dialog open={open} onClose={close} labelledBy={titleId} className="lumen-plnew">
      {/* Header — frame title #8F8F8F + the ✕ close (Icon Wrapper at the
          frame's right inset). */}
      <header className="lumen-plnew__head">
        <h2 id={titleId} className="lumen-plnew__title">
          {m("playlist.new.title")}
        </h2>
        <IconButton label={m("playlist.new.close")} size="sm" onClick={close}>
          <Icon name="close" size={16} />
        </IconButton>
      </header>

      {/* Playlist details — label + helper ride ABOVE the input row (frame
          Frame 327/328); the input reuses the shared field control grammar. */}
      <section className="lumen-plnew__section">
        <label className="lumen-plnew__label" htmlFor={`${titleId}-input`}>
          {m("playlist.new.details")}
        </label>
        <p className="lumen-plnew__helper" id={helperId}>
          {m("playlist.new.details.helper")}
        </p>
        <div className="lumen-plnew__row">
          <input
            ref={titleInputRef}
            id={`${titleId}-input`}
            className="lumen-field__control lumen-plnew__input"
            type="text"
            placeholder={m("playlist.new.name-placeholder")}
            aria-describedby={helperId}
            value={title}
            onChange={(event) => setTitle(event.target.value)}
          />
          {/* Thumbnail picker — disabled-honest: no thumbnail backend
              exists (title hint carries the reason, the a10 create rule). */}
          <button
            type="button"
            className="lumen-plnew__thumb"
            disabled
            title={m("playlist.new.thumbnail.hint")}
            aria-label={m("playlist.new.thumbnail")}
          >
            <span className="lumen-plnew__thumb-tile" aria-hidden="true">
              <Icon name="add" size={16} />
            </span>
          </button>
        </div>
      </section>

      <Separator className="lumen-plnew__rule" />

      {/* Select files — the drop zone is a <label> for the hidden input:
          clicking anywhere (or dropping) records the file names. */}
      <section className="lumen-plnew__section">
        <span className="lumen-plnew__label">{m("playlist.new.files")}</span>
        <p className="lumen-plnew__helper">{m("playlist.new.files.helper")}</p>
        <label
          className={`lumen-plnew__drop${dropOver ? " lumen-plnew__drop--over" : ""}`}
          htmlFor={fileInputId}
          onDragOver={(event) => {
            event.preventDefault();
            setDropOver(true);
          }}
          onDragLeave={() => setDropOver(false)}
          onDrop={onDrop}
        >
          <input
            ref={fileInputRef}
            id={fileInputId}
            type="file"
            multiple
            className="lumen-plnew__file-input"
            aria-label={m("playlist.new.browse")}
            onChange={onFileChange}
          />
          <span className="lumen-plnew__drop-text">{m("playlist.new.drop")}</span>
          <span className="lumen-plnew__drop-tile" aria-hidden="true">
            <Icon name="add" size={16} />
          </span>
        </label>
        {/* Recorded names — honest feedback (the browse view is the
            thumbnail/media lane's). */}
        {fileNames.length > 0 ? (
          <p className="lumen-plnew__files-count">
            {String(fileNames.length)} {m("playlist.library.items-suffix")}
          </p>
        ) : null}
      </section>

      {/* Footer — Cancel left, Save right (frame Frame 225); Save is the
          tinted confirm and stays disabled until the title is non-empty
          (the frame's default state: the DIMMED tinted pill, w26.gfix-E). */}
      <footer className="lumen-plnew__actions">
        <Button className="lumen-plnew__cancel" onClick={close}>
          {m("playlist.new.cancel")}
        </Button>
        <Button variant="tinted" disabled={title.trim() === ""} onClick={save}>
          {m("playlist.new.save")}
        </Button>
      </footer>
    </Dialog>
  );
}
