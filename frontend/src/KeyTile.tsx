import { useEffect, useState } from 'react';
import { KeyTileContent } from './KeyTileContent';
import type { KeyConfig, PagePath } from './types';
import { type KeyTileDragHandlers, useKeyTileDrag } from './useKeyTileDrag';

interface KeyTileProps {
  id: number;
  path: PagePath;
  config: KeyConfig;
  version: number;
  selected: boolean;
  /** True briefly when this key was just physically pressed on the device. */
  flashed: boolean;
  onClick: (id: number) => void;
  drag: KeyTileDragHandlers;
}

export function KeyTile({
  id,
  path,
  config,
  version,
  selected,
  flashed,
  onClick,
  drag,
}: KeyTileProps) {
  const [broken, setBroken] = useState(false);

  useEffect(() => setBroken(false), [config.icon, version]);

  // Key 0 is reserved on every non-home page to go up a level.
  const isBackKey = id === 0 && path.length > 0;
  // The server falls back to a default icon for folders, so they always
  // have an image to show here.
  const showImage =
    !isBackKey && (config.icon !== undefined || config.is_folder) && !broken;

  const { dragOver, ...dragHandlers } = useKeyTileDrag({
    id,
    isBackKey,
    isFolder: config.is_folder,
    ...drag,
  });

  return (
    <button
      type="button"
      className={`relative aspect-square cursor-pointer overflow-hidden rounded-[10px] border p-0 hover:border-accent-border ${
        selected || dragOver ? 'border-accent-border' : 'border-border'
      } ${flashed ? 'ring-2 ring-accent-border' : ''} bg-code-bg`}
      onClick={() => onClick(id)}
      {...dragHandlers}
    >
      <span className="pointer-events-none absolute top-1 left-1.5 z-1 font-mono text-xs text-text-h opacity-60">
        {id}
      </span>
      <KeyTileContent
        id={id}
        path={path}
        version={version}
        isBackKey={isBackKey}
        showImage={showImage}
        isFolder={config.is_folder}
        onImageError={() => setBroken(true)}
      />
      {!isBackKey && config.title && (
        <span className="pointer-events-none absolute inset-x-0 bottom-0 truncate bg-black/60 px-1 py-0.5 text-[10px] text-white">
          {config.title}
        </span>
      )}
    </button>
  );
}
