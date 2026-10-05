/**
 * Picture: Workshop's panel for the room's picture (RoomPicture.tsx),
 * filling the panel at whatever size the player gives it. Immersive has
 * the picture in Here instead (ScenePanel.tsx).
 */
import type { Scene } from "../lib/mud";
import { noPictureWords, type PictureSettings } from "../lib/pictures";
import { RoomPicture } from "./RoomPicture";

interface PicturePanelProps {
  scene: Scene | null;
  settings: PictureSettings;
  /** Whether the "when" setting has the picture show in this room. */
  shows: boolean;
}

export function PicturePanel({ scene, settings, shows }: PicturePanelProps) {
  return (
    <section className="picture-panel panel" aria-labelledby="picture-title" data-testid="picture-panel">
      <h2 id="picture-title">Picture</h2>
      <div className="picture-body">
        {settings.engine === "none" ? (
          <p className="map-muted">Pictures are off. Gear, then Pictures, turns them back on.</p>
        ) : scene && shows ? (
          <RoomPicture scene={scene} settings={settings} />
        ) : scene ? (
          <p className="map-muted">{noPictureWords(settings)}</p>
        ) : (
          <p className="map-muted">Once you're in the game, a picture of the room you're in shows here.</p>
        )}
      </div>
    </section>
  );
}
