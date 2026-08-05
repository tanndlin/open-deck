import { keyImageUrl } from './api';
import backArrowIcon from './assets/back_arrow.png';
import type { PagePath } from './types';

interface KeyTileContentProps {
  id: number;
  path: PagePath;
  version: number;
  isBackKey: boolean;
  showImage: boolean;
  isFolder: boolean;
  onImageError: () => void;
}

export function KeyTileContent({
  id,
  path,
  version,
  isBackKey,
  showImage,
  isFolder,
  onImageError,
}: KeyTileContentProps) {
  if (isBackKey) {
    return (
      <img
        className="pointer-events-none block h-full w-full object-cover p-3"
        src={backArrowIcon}
        alt="Back"
      />
    );
  }

  if (showImage) {
    return (
      <img
        className="pointer-events-none block h-full w-full object-contain"
        src={keyImageUrl(path, id, version)}
        alt={`Key ${id}`}
        onError={onImageError}
      />
    );
  }

  if (isFolder) {
    return (
      <span className="pointer-events-none flex h-full w-full items-center justify-center text-2xl">
        📁
      </span>
    );
  }

  return <span className="pointer-events-none block h-full w-full" />;
}
