import { useState, useEffect, useCallback } from 'react';
import { ImageUp, ClipboardPaste, Camera, Scan, AlertCircle, ArrowRight } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useNavigate } from 'react-router-dom';
import { useFumenStore } from '@/stores/fumenStore';
import { Field, decoder } from 'tetris-fumen';

const PIECE_CHARS = new Set(['I', 'O', 'T', 'S', 'Z', 'J', 'L', 'X']);

function parseFieldToFumenPages(fieldStr: string) {
  const lines = fieldStr.trim().split('\n').filter(Boolean);
  if (lines.length === 0 || lines[0].length !== 10) return null;
  
  try {
    const field = Field.create('_'.repeat(10 * 23), '_'.repeat(10));
    for (let row = 0; row < lines.length; row++) {
      const line = lines[lines.length - 1 - row];
      for (let col = 0; col < 10; col++) {
        const ch = line[col];
        if (PIECE_CHARS.has(ch)) {
          field.set(col, row, ch as any);
        }
      }
    }
    const encoded = field.to_fumen_string();
    return decoder.decode(encoded.startsWith('v115@') ? encoded : `v115@${encoded}`);
  } catch {
    return null;
  }
}

export default function RecognizePage() {
  const navigate = useNavigate();
  const setPages = useFumenStore((s) => s.setPages);
  const [imageDataUrl, setImageDataUrl] = useState<string | null>(null);
  const [imageBytes, setImageBytes] = useState<number[] | null>(null);
  const [imagePath, setImagePath] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fieldStr, setFieldStr] = useState<string | null>(null);
  const [fieldLines, setFieldLines] = useState<string[]>([]);

  // Handle clipboard paste
  useEffect(() => {
    const handler = (e: ClipboardEvent) => {
      const items = e.clipboardData?.items;
      if (!items) return;
      for (const item of items) {
        if (item.type.startsWith('image/')) {
          e.preventDefault();
          const blob = item.getAsFile();
          if (!blob) continue;
          const reader = new FileReader();
          reader.onload = async () => {
            const dataUrl = reader.result as string;
            setImageDataUrl(dataUrl);
            setImagePath(null);
            setFieldStr(null);
            setError(null);
            // Pre-load as bytes for Rust
            const resp = await fetch(dataUrl);
            const buf = await resp.arrayBuffer();
            setImageBytes(Array.from(new Uint8Array(buf)));
          };
          reader.readAsDataURL(blob);
          break;
        }
      }
    };
    document.addEventListener('paste', handler);
    return () => document.removeEventListener('paste', handler);
  }, []);

  const handleFileSelect = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'bmp', 'webp', 'gif'] }],
      });
      if (!selected) return;
      
      // Show preview via data URL
      const resp = await fetch(selected);
      const blob = await resp.blob();
      const reader = new FileReader();
      reader.onload = () => {
        setImageDataUrl(reader.result as string);
        setImagePath(selected);
        setImageBytes(null);
        setFieldStr(null);
        setError(null);
      };
      reader.readAsDataURL(blob);
    } catch (err) {
      setError(`Failed to open file: ${err}`);
    }
  }, []);

  const handleCapture = useCallback(async () => {
    setLoading(true);
    setError(null);
    setFieldStr(null);
    try {
      const result = await invoke<string>('capture_and_recognize');
      setFieldStr(result);
      setFieldLines(result.split('\n').filter(Boolean));
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const handleRecognize = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      let result: string;
      if (imagePath) {
        result = await invoke<string>('recognize_field_from_path', { path: imagePath });
      } else if (imageBytes) {
        result = await invoke<string>('recognize_field_from_bytes', { bytes: imageBytes });
      } else {
        throw new Error('No image available');
      }
      setFieldStr(result);
      setFieldLines(result.split('\n').filter(Boolean));
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [imagePath, imageBytes]);

  const handleApplyToEditor = useCallback(() => {
    if (!fieldStr) return;
    const pages = parseFieldToFumenPages(fieldStr);
    if (!pages) return;
    setPages(pages);
    navigate('/fumen-editor');
  }, [fieldStr, setPages, navigate]);

  const lines = fieldLines;
  const hasContent = lines.some(l => l.split('').some(c => PIECE_CHARS.has(c)));
  const hasImage = !!(imageDataUrl);

  return (
    <div className="max-w-3xl mx-auto space-y-4">
      <div className="space-y-1">
        <h2 className="text-xl font-semibold">Screenshot Recognition</h2>
        <p className="text-sm text-muted-foreground">
          Take a screenshot or paste an image of a Tetris board to recognize the field.
        </p>
      </div>

      {/* Actions */}
      <div className="flex gap-2">
        <button
          onClick={handleFileSelect}
          className="flex items-center gap-1.5 rounded-md bg-primary/15 px-3 py-1.5 text-xs font-medium text-primary hover:bg-primary/25 transition-colors"
        >
          <ImageUp className="h-3.5 w-3.5" />
          Open Screenshot
        </button>
        <button
          onClick={handleCapture}
          disabled={loading}
          className="flex items-center gap-1.5 rounded-md bg-primary/15 px-3 py-1.5 text-xs font-medium text-primary hover:bg-primary/25 transition-colors disabled:opacity-50"
        >
          <Camera className={`h-3.5 w-3.5 ${loading ? 'animate-pulse' : ''}`} />
          Capture Screen
        </button>
        <div className="flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-xs text-muted-foreground">
          <ClipboardPaste className="h-3.5 w-3.5" />
          Paste (Ctrl+V)
        </div>
      </div>

      {/* Image preview */}
      {hasImage && (
        <div className="space-y-3">
          <div className="rounded-lg border border-border overflow-hidden max-h-[400px]">
            <img
              src={imageDataUrl!}
              alt="Screenshot"
              className="w-full h-auto object-contain bg-black/20"
              style={{ maxHeight: 400 }}
            />
          </div>
          {!fieldStr && (
            <button
              onClick={handleRecognize}
              disabled={loading}
              className="w-full flex items-center justify-center gap-1.5 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-50"
            >
              <Scan className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
              {loading ? 'Recognizing...' : 'Recognize Field'}
            </button>
          )}
        </div>
      )}

      {/* Empty state */}
      {!hasImage && !fieldStr && (
        <div className="flex flex-col items-center justify-center rounded-lg border-2 border-dashed border-border py-16 text-center">
          <Scan className="h-10 w-10 text-muted-foreground/40 mb-3" />
          <p className="text-sm text-muted-foreground">Open a screenshot or press Ctrl+V to paste</p>
        </div>
      )}

      {/* Error */}
      {error && (
        <div className="flex items-start gap-2 rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-400">
          <AlertCircle className="h-3.5 w-3.5 mt-0.5 shrink-0" />
          <span>{error}</span>
        </div>
      )}

      {/* Recognized field */}
      {fieldStr && (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-xs font-medium text-muted-foreground">
              Recognized Field ({lines.length} rows × 10 cols{hasContent ? ', pieces detected' : ' — appears empty'})
            </span>
            {!hasContent && (
              <span className="text-[10px] text-yellow-400">⚠️ May need adjustment</span>
            )}
          </div>
          
          <div className="rounded-lg border border-border overflow-hidden">
            <div className="p-3 font-mono text-xs leading-5 tracking-wider bg-background">
              {lines.map((line, idx) => (
                <div key={idx} className="flex">
                  <span className="text-[10px] text-muted-foreground w-5 text-right mr-1 shrink-0 select-none">
                    {lines.length - idx}
                  </span>
                  <span className="text-foreground/90">
                    {line.split('').map((ch, ci) => {
                      const colors: Record<string, string> = {
                        I: 'text-cyan-400',
                        O: 'text-yellow-400',
                        T: 'text-purple-400',
                        S: 'text-green-400',
                        Z: 'text-red-400',
                        J: 'text-blue-400',
                        L: 'text-orange-400',
                        X: 'text-gray-400',
                      };
                      return (
                        <span key={ci} className={colors[ch] || 'text-foreground/30'}>
                          {ch === '_' ? '·' : ch}
                        </span>
                      );
                    })}
                  </span>
                </div>
              ))}
            </div>
          </div>

          {/* Raw field string (editable) */}
          <div className="space-y-1">
            <label className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
              Raw Field (edit if needed)
            </label>
            <textarea
              value={fieldStr}
              onChange={(e) => {
                setFieldStr(e.target.value);
                setFieldLines(e.target.value.split('\n').filter(Boolean));
              }}
              className="w-full rounded-md border border-input bg-background px-2.5 py-1.5 font-mono text-xs min-h-[80px] resize-y focus:outline-none focus:ring-1 focus:ring-ring"
              spellCheck={false}
            />
          </div>

          {hasContent && (
            <button
              onClick={handleApplyToEditor}
              className="flex items-center justify-center gap-1.5 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 transition-colors w-full"
            >
              <ArrowRight className="h-4 w-4" />
              Send to Fumen Editor
            </button>
          )}
        </div>
      )}
    </div>
  );
}
