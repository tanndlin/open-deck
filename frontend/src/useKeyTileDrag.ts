import { type DragEvent, useState } from 'react';

const IMAGE_FILE_NAME_RE = /\.(png|jpe?g|gif|bmp|webp|ico|svg)$/i;

/** `File.type` is often empty for less common formats (e.g. `.ico`), so fall back to the extension. */
function isImageFile(file: File): boolean {
  return file.type.startsWith('image/') || IMAGE_FILE_NAME_RE.test(file.name);
}

export interface KeyTileDragHandlers {
  onDragStart: (id: number) => void;
  onDragEnd: () => void;
  onDropKey: (id: number) => void;
  onDropFile: (id: number, file: File) => void;
  onHoverFolder: (id: number) => void;
  onHoverBack: () => void;
  onHoverCancel: () => void;
}

interface UseKeyTileDragParams extends KeyTileDragHandlers {
  id: number;
  isBackKey: boolean;
  isFolder: boolean;
}

/** Wires up the drag-source and drop-target behavior shared by every key tile. */
export function useKeyTileDrag({
  id,
  isBackKey,
  isFolder,
  onDragStart,
  onDragEnd,
  onDropKey,
  onDropFile,
  onHoverFolder,
  onHoverBack,
  onHoverCancel,
}: UseKeyTileDragParams) {
  const [dragOver, setDragOver] = useState(false);

  return {
    dragOver,
    draggable: !isBackKey,
    onDragStart: (e: DragEvent) => {
      if (isBackKey) return;
      e.dataTransfer.effectAllowed = 'move';
      onDragStart(id);
    },
    onDragEnd,
    onDragOver: (e: DragEvent) => e.preventDefault(),
    onDragEnter: (e: DragEvent) => {
      e.preventDefault();
      setDragOver(true);
      if (isBackKey) onHoverBack();
      else if (isFolder) onHoverFolder(id);
    },
    onDragLeave: () => {
      setDragOver(false);
      onHoverCancel();
    },
    onDrop: (e: DragEvent) => {
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
    },
  };
}
