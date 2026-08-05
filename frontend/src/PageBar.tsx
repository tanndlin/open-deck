import type { PagePath } from './api';

interface PageBarProps {
  path: PagePath;
  currentDevicePath: PagePath | null;
  onNavigate: (path: PagePath) => void;
}

function samePath(a: PagePath, b: PagePath): boolean {
  return a.length === b.length && a.every((v, i) => v === b[i]);
}

export function PageBar({ path, currentDevicePath, onNavigate }: PageBarProps) {
  const isOnDevice =
    currentDevicePath !== null && samePath(path, currentDevicePath);

  return (
    <div className="mx-auto flex w-full max-w-160 flex-wrap items-center justify-center gap-1 text-sm">
      <button
        type="button"
        className="cursor-pointer rounded-sm border-none bg-transparent px-1.5 py-0.5 text-text-h disabled:cursor-default disabled:opacity-100 disabled:underline"
        disabled={path.length === 0}
        onClick={() => onNavigate([])}
      >
        Home
      </button>
      {path.map((key, i) => (
        <span key={i} className="flex items-center gap-1">
          <span className="text-text opacity-50">/</span>
          <button
            type="button"
            className="cursor-pointer rounded-sm border-none bg-transparent px-1.5 py-0.5 text-text-h disabled:cursor-default disabled:opacity-100 disabled:underline"
            disabled={i === path.length - 1}
            onClick={() => onNavigate(path.slice(0, i + 1))}
          >
            Key {key}
          </button>
        </span>
      ))}
      {!isOnDevice && currentDevicePath !== null && (
        <span className="ml-1 text-xs text-text opacity-60">
          (device is showing a different page)
        </span>
      )}
    </div>
  );
}
