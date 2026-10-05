/**
 * Here: Immersive mode's panel, and Workshop's to show. The room, its
 * picture (Immersive's; Workshop has a Picture panel), a compass of its exits, the character's health, mana and movement, the
 * fight, and which sound layers are playing: what the cues say, for
 * anyone who can glance at it, so a sighted player can learn the sounds
 * by seeing them beside it.
 *
 * Words first (rule 10): the compass and the bars are hidden from
 * assistive technology, and the same facts are beside them as text.
 */
import { useEffect, useState } from "react";
import * as sound from "../lib/assets";
import * as mud from "../lib/mud";
import { noPictureWords, type PictureSettings } from "../lib/pictures";
import { terrainWords } from "../lib/terrain";
import { Fight, Vitals } from "./Gauges";
import { RoomPicture } from "./RoomPicture";

interface ScenePanelProps {
  snapshot: mud.MapSnapshot | null;
  vitals: mud.Vitals | null;
  opponent: mud.Opponent | null;
  /** The music playing, as an asset path. */
  music: string | null;
  cues: boolean;
  voice: boolean;
  /** The room's picture, under its name: Immersive's, with pictures on. `shows` is false where the "when" setting leaves it out. */
  picture?: { scene: mud.Scene; settings: PictureSettings; shows: boolean } | null;
}

/** How tall the picture is in Here, in rows. */
const PICTURE_ROWS = 12;

/** Each exit as the compass shows it: its letters, then ? not explored, + a closed door, # locked. */
function mark(exit: mud.MapExit | undefined): string {
  if (!exit) return "  ";
  const tag = exit.locked ? "#" : exit.door && !exit.open ? "+" : !exit.visited ? "?" : "";
  return `${exit.dir}${tag}`.padEnd(3).slice(0, 3);
}

function compass(room: mud.MapRoom): string[] {
  const at = (dir: string) => room.exits.find((e) => e.dir === dir);
  const cell = (dir: string) => mark(at(dir)).padEnd(4);
  return [
    `${cell("NW")}${cell("N")}${cell("NE")}  ${cell("U")}`,
    `${cell("W")}${"@".padEnd(4)}${cell("E")}`,
    `${cell("SW")}${cell("S")}${cell("SE")}  ${cell("D")}`,
  ];
}

export function ScenePanel({ snapshot, vitals, opponent, music, cues, voice, picture }: ScenePanelProps) {
  const room = snapshot?.room ?? null;
  const [loops, setLoops] = useState<Record<sound.AmbientBus, string | null>>({ bgn: null, bgw: null });
  useEffect(() => sound.onLoops(setLoops), []);
  const layer = (name: string, playing: string | null | boolean) => (
    <li>
      <span className="scene-label">{name.padEnd(8)}</span>
      {typeof playing === "boolean" ? (playing ? "on" : "off") : playing ? mud.assetTitle(playing) : "silent"}
    </li>
  );

  return (
    <section className="scene-panel panel" aria-labelledby="scene-title" data-testid="scene-panel">
      <h2 id="scene-title">Here</h2>
      <div className="scene-body">
        {room ? (
          <p className="scene-room">
            <span className="map-room-name">{room.landmark ? `${room.name} (${room.landmark})` : room.name}</span>
            <br />
            <span className="map-muted">
              {room.zone}
              {room.terrain ? `, ${terrainWords(room.terrain)}` : ""}
            </span>
          </p>
        ) : (
          <p className="map-muted">Once you're in the game, the room you're in shows here, and its exits are heard as you arrive.</p>
        )}
        {picture &&
          (picture.shows ? (
            <RoomPicture scene={picture.scene} settings={picture.settings} rows={PICTURE_ROWS} />
          ) : (
            // Its place is kept, so nothing under it moves from room to room.
            <div className="room-picture room-picture-none" style={{ height: (PICTURE_ROWS + 1) * 16 }} aria-hidden="true">
              <p className="map-muted">{noPictureWords(picture.settings)}</p>
            </div>
          ))}
        {room && (
          <>
            <pre className="scene-compass" aria-hidden="true" data-testid="scene-compass">
              {compass(room).join("\n")}
            </pre>
            <p className="map-muted" aria-hidden="true">
              ? not explored · + door · # locked
            </p>
            <p className="sr-only">{mud.describeExits(room)}</p>
          </>
        )}

        <h3 className="map-heading">You</h3>
        <Vitals vitals={vitals} />

        <h3 className="map-heading">Fight</h3>
        <Fight opponent={opponent} />

        <h3 className="map-heading">Sound layers</h3>
        <ul className="scene-layers" data-testid="scene-layers">
          {layer("Place", loops.bgn)}
          {layer("Sky", loops.bgw)}
          {layer("Music", music)}
          {layer("Cues", cues)}
          {layer("Voice", voice)}
        </ul>
      </div>
    </section>
  );
}
