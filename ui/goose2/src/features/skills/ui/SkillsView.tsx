import { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  ArrowRight,
  ArrowUpDown,
  List,
  Plus,
  Trash2,
  MoreHorizontal,
  Pencil,
  Copy,
  Download,
  Upload,
} from "lucide-react";
import { cn } from "@/shared/lib/cn";
import { Button, buttonVariants } from "@/shared/ui/button";
import { useSetTopBarActions } from "@/app/contexts/TopBarActionsContext";
import { BottomFade } from "@/shared/ui/BottomFade";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/shared/ui/alert-dialog";
import { useFileImportZone } from "@/shared/hooks/useFileImportZone";
import { CreateSkillDialog } from "./CreateSkillDialog";
import {
  listSkills,
  deleteSkill,
  createSkill,
  exportSkill,
  importSkills,
  type SkillInfo,
} from "../api/skills";

const TAG_PILL_COLORS = [
  "var(--pill-pink)",
  "var(--pill-olive)",
  "var(--pill-blue)",
] as const;

function tagPillColor(index: number): string {
  return TAG_PILL_COLORS[index % TAG_PILL_COLORS.length];
}

function SkillCardMenu({
  skill,
  onEdit,
  onDuplicate,
  onExport,
  onDelete,
}: {
  skill: SkillInfo;
  onEdit: (skill: SkillInfo) => void;
  onDuplicate: (skill: SkillInfo) => void;
  onExport: (skill: SkillInfo) => void;
  onDelete: (skill: SkillInfo) => void;
}) {
  const { t } = useTranslation(["skills", "common"]);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon-xs"
          aria-label={t("view.optionsAria", { name: skill.name })}
          className="size-6 rounded-md text-muted-foreground hover:text-foreground"
        >
          <MoreHorizontal className="size-3.5" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" sideOffset={4}>
        <DropdownMenuItem onSelect={() => onEdit(skill)}>
          <Pencil className="size-3.5" />
          {t("common:actions.edit")}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => onDuplicate(skill)}>
          <Copy className="size-3.5" />
          {t("common:actions.duplicate")}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => onExport(skill)}>
          <Download className="size-3.5" />
          {t("common:actions.export")}
        </DropdownMenuItem>
        <DropdownMenuItem
          variant="destructive"
          onSelect={() => onDelete(skill)}
        >
          <Trash2 className="size-3.5" />
          {t("common:actions.delete")}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export function SkillsView() {
  const { t } = useTranslation(["skills", "common"]);
  const setTopBarActions = useSetTopBarActions();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingSkill, setEditingSkill] = useState<
    { name: string; description: string; instructions: string } | undefined
  >(undefined);
  const [skills, setSkills] = useState<SkillInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [deletingSkill, setDeletingSkill] = useState<SkillInfo | null>(null);
  const [notification, setNotification] = useState<string | null>(null);
  const importInputRef = useRef<HTMLInputElement>(null);

  const loadSkills = useCallback(async () => {
    try {
      const result = await listSkills();
      setSkills(result);
    } catch {
      // skills directory may not exist yet
      setSkills([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadSkills();
  }, [loadSkills]);

  const handleDelete = (skill: SkillInfo) => {
    setDeletingSkill(skill);
  };

  const handleConfirmDeleteSkill = async () => {
    if (!deletingSkill) return;
    try {
      await deleteSkill(deletingSkill.name);
      await loadSkills();
    } catch {
      // best-effort
    }
    setDeletingSkill(null);
  };

  const handleEdit = (skill: SkillInfo) => {
    setEditingSkill({
      name: skill.name,
      description: skill.description,
      instructions: skill.instructions,
    });
    setDialogOpen(true);
  };

  const handleDuplicate = async (skill: SkillInfo) => {
    const existingNames = new Set(skills.map((s) => s.name));
    let copyName = `${skill.name}-copy`;
    let counter = 2;
    while (existingNames.has(copyName)) {
      copyName = `${skill.name}-copy-${counter}`;
      counter++;
    }
    try {
      await createSkill(copyName, skill.description, skill.instructions);
      await loadSkills();
    } catch {
      // best-effort
    }
  };

  const handleExport = async (skill: SkillInfo) => {
    try {
      const result = await exportSkill(skill.name);
      const blob = new Blob([result.json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = result.filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      setNotification(t("view.exportedTo", { filename: result.filename }));
      setTimeout(() => setNotification(null), 3000);
    } catch (err) {
      console.error("Failed to export skill:", err);
    }
  };

  const handleImportFile = useCallback(
    async (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (!file) return;

      try {
        const arrayBuffer = await file.arrayBuffer();
        const bytes = Array.from(new Uint8Array(arrayBuffer));
        await importSkills(bytes, file.name);
        await loadSkills();
      } catch (err) {
        console.error("Failed to import skill:", err);
      }

      // Reset the input so the same file can be re-selected
      if (importInputRef.current) {
        importInputRef.current.value = "";
      }
    },
    [loadSkills],
  );

  const handleDialogClose = () => {
    setDialogOpen(false);
    setEditingSkill(undefined);
  };

  const handleNewSkill = useCallback(() => {
    setEditingSkill(undefined);
    setDialogOpen(true);
  }, []);

  const handleDropImport = useCallback(
    async (fileBytes: number[], fileName: string) => {
      try {
        await importSkills(fileBytes, fileName);
        await loadSkills();
      } catch (err) {
        console.error("Failed to import skill:", err);
      }
    },
    [loadSkills],
  );

  const {
    fileInputRef: dropFileInputRef,
    isDragOver,
    dropHandlers,
    handleFileChange: handleDropFileChange,
  } = useFileImportZone({ onImportFile: handleDropImport });

  useEffect(() => {
    const pillCls =
      "h-8 rounded-full bg-[var(--surface-button)] px-3 text-[14px] text-black/70 hover:bg-[var(--surface-button)]/80";
    const iconBtnCls =
      "h-8 w-9 rounded-full bg-[var(--surface-button)] p-0 text-black/70 hover:bg-[var(--surface-button)]/80";
    setTopBarActions(
      <>
        <Button
          type="button"
          variant="ghost"
          className={iconBtnCls}
          aria-label={t("view.listView")}
          /* i18n-check-ignore: aria-label resolved via t() above */
          tabIndex={-1}
        >
          <List className="size-4" />
        </Button>
        <Button
          type="button"
          variant="ghost"
          className={iconBtnCls}
          aria-label={t("view.sort")}
          /* i18n-check-ignore: aria-label resolved via t() above */
          tabIndex={-1}
        >
          <ArrowUpDown className="size-4" />
        </Button>
        <Button
          type="button"
          variant="ghost"
          className={pillCls}
          onClick={() => importInputRef.current?.click()}
        >
          <Upload className="mr-2 size-4" />
          {t("common:actions.import")}
        </Button>
        <Button
          type="button"
          variant="ghost"
          className={pillCls}
          onClick={handleNewSkill}
        >
          <Plus className="mr-2 size-4" />
          {t("view.newSkill")}
        </Button>
      </>,
    );
    return () => setTopBarActions(null);
  }, [setTopBarActions, t, handleNewSkill]);

  return (
    <div className="flex flex-1 flex-col h-full min-h-0">
      <div
        className="flex-1 overflow-y-auto min-h-0 relative"
        {...dropHandlers}
      >
        <div className="max-w-7xl mx-auto w-full px-6 py-8 page-transition">
          <input
            ref={importInputRef}
            type="file"
            accept=".skill.json,.json"
            className="hidden"
            onChange={handleImportFile}
          />

          {/* Skills grid (always shows new-skill card; cards appear when present) */}
          {!loading && (
            <section
              className={cn(
                "grid grid-cols-[repeat(auto-fill,260px)] gap-8 rounded-tile transition-colors",
                isDragOver && "ring-2 ring-ring ring-offset-2",
              )}
            >
              {/* New-skill empty state — always first */}
              <div className="group relative h-[260px] w-[260px] overflow-hidden rounded-tile bg-[var(--surface-tile)] p-5">
                <span className="inline-flex h-5 items-center rounded-full bg-[var(--pill-neutral)] px-[6px] pb-[3px] text-[14px] text-[var(--text-title-alex)]">
                  {/* i18n-check-ignore: visual placeholder pill mirroring the kebab-case skill-name pattern, not user copy */}
                  new-skill
                </span>
                <p className="mt-8 text-[16px] leading-[20px] text-[var(--text-muted-alex)]">
                  {t("view.newSkillEmptyExample")}{" "}
                  <em className="italic">
                    {t("view.newSkillEmptyExampleQuote")}
                  </em>
                </p>
                <button
                  type="button"
                  onClick={handleNewSkill}
                  className="absolute bottom-4 right-4 flex h-8 w-10 items-center justify-center rounded-full bg-[var(--surface-install)]"
                  aria-label={t("view.newSkill")}
                >
                  <ArrowRight className="size-4 text-black/70" />
                </button>
              </div>

              {skills.map((skill, index) => (
                <div
                  key={skill.name}
                  className="group relative h-[260px] w-[260px] overflow-hidden rounded-tile bg-[var(--surface-tile)] p-5"
                >
                  <span
                    className="inline-flex h-5 items-center rounded-full px-[6px] pb-[3px] text-[14px] text-[var(--text-title-alex)]"
                    style={{ backgroundColor: tagPillColor(index) }}
                  >
                    {skill.name}
                  </span>

                  <p
                    className="mt-8 text-[16px] leading-[20px] text-[var(--text-muted-alex)]"
                    style={{
                      display: "-webkit-box",
                      WebkitBoxOrient: "vertical",
                      WebkitLineClamp: 7,
                      overflow: "hidden",
                    }}
                  >
                    {skill.description}
                  </p>

                  {/* Hover-only Install — visual placeholder; no real install flow exists for user-created skills */}
                  <button
                    type="button"
                    className="absolute bottom-4 right-4 hidden h-8 rounded-full bg-[var(--surface-install)] px-3 text-[14px] text-black/70 group-hover:inline-flex group-hover:items-center"
                    /* i18n-check-ignore: decorative placeholder button — no real install flow */
                    aria-label={`Install ${skill.name} (placeholder)`}
                    tabIndex={-1}
                  >
                    {/* i18n-check-ignore: decorative placeholder button — no real install flow */}
                    Install
                  </button>

                  {/* Existing menu, hover-revealed */}
                  <div className="absolute right-4 top-4 hidden group-hover:block">
                    <SkillCardMenu
                      skill={skill}
                      onEdit={handleEdit}
                      onDuplicate={handleDuplicate}
                      onExport={handleExport}
                      onDelete={handleDelete}
                    />
                  </div>
                </div>
              ))}
            </section>
          )}
        </div>

        <BottomFade />
      </div>

      {/* Hidden file input for drag-and-drop import */}
      <input
        ref={dropFileInputRef}
        type="file"
        accept=".skill.json,.json"
        className="hidden"
        onChange={handleDropFileChange}
      />

      {/* Create / Edit dialog */}
      <CreateSkillDialog
        isOpen={dialogOpen}
        onClose={handleDialogClose}
        onCreated={loadSkills}
        editingSkill={editingSkill}
      />

      {/* Delete confirmation dialog */}
      <AlertDialog
        open={!!deletingSkill}
        onOpenChange={(open) => !open && setDeletingSkill(null)}
      >
        <AlertDialogContent className="max-w-sm">
          <AlertDialogHeader>
            <AlertDialogTitle>{t("view.deleteTitle")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("view.deleteDescription", {
                name: deletingSkill?.name ?? "",
              })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common:actions.cancel")}</AlertDialogCancel>
            <AlertDialogAction
              className={buttonVariants({ variant: "destructive" })}
              onClick={handleConfirmDeleteSkill}
            >
              {t("common:actions.delete")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Export notification toast */}
      {notification && (
        <div className="fixed bottom-4 right-4 z-50 rounded-lg border border-border bg-background px-4 py-3 shadow-popover text-sm animate-in fade-in slide-in-from-bottom-2">
          {notification}
        </div>
      )}
    </div>
  );
}
