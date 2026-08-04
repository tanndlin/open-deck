import { useEffect, useState } from 'react';
import { keyImageUrl } from './api';

interface KeyTileProps {
  id: number;
  path?: string;
  version: number;
  onClick: (id: number) => void;
}

export function KeyTile({ id, path, version, onClick }: KeyTileProps) {
  const [broken, setBroken] = useState(false);

  useEffect(() => setBroken(false), [path, version]);

  const showImage = path !== undefined && !broken;

  return (
    <button
      type="button"
      className="relative aspect-square cursor-pointer overflow-hidden rounded-[10px] border border-border bg-code-bg p-0 hover:border-accent-border"
      onClick={() => onClick(id)}
    >
      <span className="absolute top-1 left-1.5 z-1 font-mono text-xs text-text-h opacity-60">
        {id}
      </span>
      {showImage ? (
        <img
          className="block h-full w-full object-cover"
          src={keyImageUrl(id, version)}
          alt={`Key ${id}`}
          onError={() => setBroken(true)}
        />
      ) : (
        <span className="block h-full w-full" />
      )}
    </button>
  );
}
