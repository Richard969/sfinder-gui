import { usePieceKeys } from '@/hooks/usePieceKeys';
import FieldGrid from '@/components/fumen/FieldGrid';
import PiecePalette from '@/components/fumen/PiecePalette';
import FumenToolbar from '@/components/fumen/FumenToolbar';
import PageNavigator from '@/components/fumen/PageNavigator';
import CommentEditor from '@/components/fumen/CommentEditor';

export default function FumenEditorPage() {
  usePieceKeys();

  return (
    <div className="flex flex-col gap-4 h-full">
      <FumenToolbar />
      <div className="flex gap-4 flex-1 min-h-0">
        <FieldGrid />
        <div className="flex flex-col gap-4 w-64 shrink-0">
          <PageNavigator />
          <PiecePalette />
          <CommentEditor />
        </div>
      </div>
    </div>
  );
}
