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
  onDragStart: (id: number) => void;
  onDragEnd: () => void;
  onDropKey: (id: number) => void;
  onHoverFolder: (id: number) => void;
  onHoverBack: () => void;
  onHoverCancel: () => void;
}

/** Every key — folder or not, on every page — renders through this exact same tile. */
export function KeyTile({
  id,
  path,
  config,
  version,
  selected,
  onClick,
  onDragStart,
  onDragEnd,
  onDropKey,
  onHoverFolder,
  onHoverBack,
  onHoverCancel,
}: KeyTileProps) {
  const [broken, setBroken] = useState(false);
  const [dragOver, setDragOver] = useState(false);

  useEffect(() => setBroken(false), [config.icon, version]);

  // Key 0 is reserved on every non-home page: pressing it always goes up a
  // level, regardless of whatever's configured there.
  const isBackKey = id === 0 && path.length > 0;
  // The server falls back to the default folder icon when a folder key has
  // none of its own set, so folders always have an image to show here.
  const showImage =
    !isBackKey && (config.icon !== undefined || config.is_folder) && !broken;

  return (
    <button
      type="button"
      className={`relative aspect-square cursor-pointer overflow-hidden rounded-[10px] border p-0 hover:border-accent-border ${
        selected || dragOver ? 'border-accent-border' : 'border-border'
      } bg-code-bg`}
      onClick={() => onClick(id)}
      draggable={!isBackKey}
      onDragStart={(e) => {
        if (isBackKey) return;
        e.dataTransfer.effectAllowed = 'move';
        onDragStart(id);
      }}
      onDragEnd={onDragEnd}
      onDragOver={(e) => e.preventDefault()}
      onDragEnter={(e) => {
        e.preventDefault();
        setDragOver(true);
        if (isBackKey) onHoverBack();
        else if (config.is_folder) onHoverFolder(id);
      }}
      onDragLeave={() => {
        setDragOver(false);
        onHoverCancel();
      }}
      onDrop={(e) => {
        e.preventDefault();
        setDragOver(false);
        onHoverCancel();
        onDropKey(id);
      }}
    >
      <span className="pointer-events-none absolute top-1 left-1.5 z-1 font-mono text-xs text-text-h opacity-60">
        {id}
      </span>
      {isBackKey ? (
        <img
          className="pointer-events-none block h-full w-full object-cover p-3"
          src={backArrowIcon}
          alt="Back"
        />
      ) : showImage ? (
        <img
          className="pointer-events-none block h-full w-full object-contain"
          src={keyImageUrl(path, id, version)}
          alt={`Key ${id}`}
          onError={() => setBroken(true)}
        />
      ) : config.is_folder ? (
        <span className="pointer-events-none flex h-full w-full items-center justify-center text-2xl">
          📁
        </span>
      ) : (
        <span className="pointer-events-none block h-full w-full" />
      )}
    </button>
  );
}
