import { useEffect, useState } from 'react';
import { getBrightness, setBrightness } from './api';

interface BrightnessSliderProps {
  setError: (error: string | null) => void;
}

export function BrightnessSlider({ setError }: BrightnessSliderProps) {
  const [brightness, setLocalBrightness] = useState<number | null>(null);

  useEffect(() => {
    getBrightness()
      .then(setLocalBrightness)
      .catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, [setError]);

  async function handleChange(percent: number) {
    setLocalBrightness(percent);
    try {
      await setBrightness(percent);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  if (brightness === null) return null;

  return (
    <label className="mx-auto flex items-center gap-3 text-sm text-text-h">
      Brightness
      <input
        type="range"
        min={0}
        max={100}
        value={brightness}
        onChange={(e) => handleChange(Number(e.target.value))}
      />
      <span className="w-10 text-left">{brightness}%</span>
    </label>
  );
}
