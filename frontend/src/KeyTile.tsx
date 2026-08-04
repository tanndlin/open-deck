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
    <button type="button" className="key-tile" onClick={() => onClick(id)}>
      <span className="key-tile-number">{id}</span>
      {showImage ? (
        <img
          className="key-tile-image"
          src={keyImageUrl(id, version)}
          alt={`Key ${id}`}
          onError={() => setBroken(true)}
        />
      ) : (
        <span className="key-tile-empty" />
      )}
    </button>
  );
}
