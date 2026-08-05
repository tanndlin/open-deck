import { useEffect, useState } from 'react';
import { keyImageUrl } from './api';
import type { KeyConfig, PagePath } from './types';
import backArrowIcon from './assets/back_arrow.png';

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
  onDropFile: (id: number, file: File) => void;
  onHoverFolder: (id: number) => void;
  onHoverBack: () => void;
  onHoverCancel: () => void;
}

const IMAGE_FILE_NAME_RE = /\.(png|jpe?g|gif|bmp|webp|ico|svg)$/i;

/** `File.type` is often empty for less common formats (e.g. `.ico`), so fall back to the extension. */
function isImageFile(file: File): boolean {
  return file.type.startsWith('image/') || IMAGE_FILE_NAME_RE.test(file.name);
}

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
  onDropFile,
  onHoverFolder,
  onHoverBack,
  onHoverCancel,
}: KeyTileProps) {
  const [broken, setBroken] = useState(false);
  const [dragOver, setDragOver] = useState(false);

  useEffect(() => setBroken(false), [config.icon, version]);

  // Key 0 is reserved on every non-home page to go up a level.
  const isBackKey = id === 0 && path.length > 0;
  // The server falls back to a default icon for folders, so they always
  // have an image to show here.
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
        if (isBackKey) return;
        const file = e.dataTransfer.files[0];
        if (file && isImageFile(file)) {
          onDropFile(id, file);
        } else {
          onDropKey(id);
        }
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
