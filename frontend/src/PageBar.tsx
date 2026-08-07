import type { PagePath } from './types';

interface PageBarProps {
  path: PagePath;
  onNavigate: (path: PagePath) => void;
}

export function PageBar({ path, onNavigate }: PageBarProps) {
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
    </div>
  );
}
