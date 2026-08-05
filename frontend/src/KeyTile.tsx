import { useEffect, useState } from 'react';
import backArrowIcon from './assets/back_arrow.png';
import { keyImageUrl, type KeyConfig, type PagePath } from './api';

interface KeyTileProps {
  id: number;
  path: PagePath;
  config: KeyConfig;
  version: number;
  selected: boolean;
  onClick: (id: number) => void;
}

/** Every key — folder or not, on every page — renders through this exact same tile. */
export function KeyTile({
  id,
  path,
  config,
  version,
  selected,
  onClick,
}: KeyTileProps) {
  const [broken, setBroken] = useState(false);

  useEffect(() => setBroken(false), [config.icon, version]);

  // Key 0 is reserved on every non-home page: pressing it always goes up a
  // level, regardless of whatever's configured there.
  const isBackKey = id === 0 && path.length > 0;
  const showImage = !isBackKey && config.icon !== undefined && !broken;

  return (
    <button
      type="button"
      className={`relative aspect-square cursor-pointer overflow-hidden rounded-[10px] border p-0 hover:border-accent-border ${
        selected ? 'border-accent-border' : 'border-border'
      } bg-code-bg`}
      onClick={() => onClick(id)}
    >
      <span className="absolute top-1 left-1.5 z-1 font-mono text-xs text-text-h opacity-60">
        {id}
      </span>
      {isBackKey ? (
        <img
          className="block h-full w-full object-cover p-3"
          src={backArrowIcon}
          alt="Back"
        />
      ) : showImage ? (
        <img
          className="block h-full w-full object-cover"
          src={keyImageUrl(path, id, version)}
          alt={`Key ${id}`}
          onError={() => setBroken(true)}
        />
      ) : config.is_folder ? (
        <span className="flex h-full w-full items-center justify-center text-2xl">
          📁
        </span>
      ) : (
        <span className="block h-full w-full" />
      )}
    </button>
  );
}
