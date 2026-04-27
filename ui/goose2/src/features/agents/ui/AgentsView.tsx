import { useState, useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { Plus, Upload } from "lucide-react";
import { toast } from "sonner";
import { Button, buttonVariants } from "@/shared/ui/button";
import { useFileImportZone } from "@/shared/hooks/useFileImportZone";
import { useSetTopBarActions } from "@/app/contexts/TopBarActionsContext";
import { BottomFade } from "@/shared/ui/BottomFade";
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
import { useAgentStore } from "@/features/agents/stores/agentStore";
import { PersonaGallery } from "@/features/agents/ui/PersonaGallery";
import { PersonaEditor } from "@/features/agents/ui/PersonaEditor";
import {
  exportPersona,
  importPersonas,
  readImportPersonaFile,
} from "@/shared/api/agents";
import { usePersonas } from "@/features/agents/hooks/usePersonas";
import type {
  Persona,
  CreatePersonaRequest,
  UpdatePersonaRequest,
} from "@/shared/types/agents";
import {
  formatAgentError,
  formatImportSuccessMessage,
  validatePersonaImportFile,
} from "@/features/agents/lib/personaImport";
import { getPersonaSource } from "@/features/agents/lib/personaPresentation";

export function AgentsView() {
  const { t } = useTranslation(["agents", "common"]);
  const setTopBarActions = useSetTopBarActions();
  const [deletingPersona, setDeletingPersona] = useState<Persona | null>(null);

  const personas = useAgentStore((s) => s.personas);
  const personasLoading = useAgentStore((s) => s.personasLoading);
  const personaEditorOpen = useAgentStore((s) => s.personaEditorOpen);
  const editingPersona = useAgentStore((s) => s.editingPersona);
  const personaEditorMode = useAgentStore((s) => s.personaEditorMode);
  const openPersonaEditor = useAgentStore((s) => s.openPersonaEditor);
  const closePersonaEditor = useAgentStore((s) => s.closePersonaEditor);

  const {
    createPersona,
    updatePersona: updatePersonaViaHook,
    deletePersona,
    refreshFromDisk,
  } = usePersonas();

  const handleSavePersona = useCallback(
    async (data: CreatePersonaRequest | UpdatePersonaRequest) => {
      try {
        if (editingPersona && personaEditorMode === "edit") {
          await updatePersonaViaHook(
            editingPersona.id,
            data as UpdatePersonaRequest,
          );
          toast.success(t("editor.updated"));
        } else {
          await createPersona(data as CreatePersonaRequest);
          toast.success(t("editor.created"));
        }
        closePersonaEditor();
      } catch (error) {
        toast.error(formatAgentError(error, t("editor.saveFailed")));
      }
    },
    [
      closePersonaEditor,
      createPersona,
      editingPersona,
      personaEditorMode,
      t,
      updatePersonaViaHook,
    ],
  );

  const handleDuplicatePersona = useCallback(
    async (persona: Persona) => {
      try {
        await createPersona({
          displayName: t("view.copyName", { name: persona.displayName }),
          avatar: persona.avatar ?? undefined,
          systemPrompt: persona.systemPrompt,
          provider: persona.provider,
          model: persona.model,
        });
        toast.success(t("editor.duplicated"));
      } catch (error) {
        toast.error(formatAgentError(error, t("editor.saveFailed")));
      }
    },
    [createPersona, t],
  );

  const handleDeletePersona = useCallback((persona: Persona) => {
    if (getPersonaSource(persona) === "builtin") return;
    setDeletingPersona(persona);
  }, []);

  const handleConfirmDeletePersona = useCallback(async () => {
    if (!deletingPersona) return;
    try {
      await deletePersona(deletingPersona.id);
      if (editingPersona?.id === deletingPersona.id) {
        closePersonaEditor();
      }
      toast.success(t("view.deleted", { name: deletingPersona.displayName }));
    } catch (err) {
      toast.error(formatAgentError(err, t("view.deleteFailed")));
    }
    setDeletingPersona(null);
  }, [closePersonaEditor, deletingPersona, deletePersona, editingPersona, t]);

  const handleExportPersona = useCallback(
    async (persona: Persona) => {
      try {
        const result = await exportPersona(persona.id);
        // Trigger a browser download with the JSON content
        const blob = new Blob([result.json], { type: "application/json" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = result.suggestedFilename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        toast.success(
          t("view.exportedTo", { filename: result.suggestedFilename }),
        );
      } catch (err) {
        toast.error(formatAgentError(err, t("view.exportFailed")));
      }
    },
    [t],
  );

  const handleImportError = useCallback((message: string) => {
    toast.error(message);
  }, []);

  const validateImportFile = useCallback(
    (file: Pick<File, "name" | "type">) => {
      const message = validatePersonaImportFile(file);
      return message ? t(message.key, message.options) : null;
    },
    [t],
  );

  const handleImportFileBytes = useCallback(
    async (fileBytes: number[], fileName: string) => {
      try {
        const imported = await importPersonas(fileBytes, fileName);
        await refreshFromDisk();
        const message = formatImportSuccessMessage(imported.length);
        toast.success(t(message.key, message.options));
      } catch (err) {
        toast.error(formatAgentError(err, t("view.importFailed")));
      }
    },
    [refreshFromDisk, t],
  );

  const handleImportPicker = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        title: t("common:actions.import"),
        filters: [
          {
            name: "JSON",
            extensions: ["json"],
          },
        ],
      });

      if (!selected || Array.isArray(selected)) {
        return;
      }

      const { fileBytes, fileName } = await readImportPersonaFile(selected);
      const validationMessage = validateImportFile({
        name: fileName,
        type: "",
      });

      if (validationMessage) {
        toast.error(validationMessage);
        return;
      }

      await handleImportFileBytes(fileBytes, fileName);
    } catch (err) {
      toast.error(formatAgentError(err, t("view.importFailed")));
    }
  }, [handleImportFileBytes, t, validateImportFile]);

  const { isDragOver, dropHandlers } = useFileImportZone({
    onImportFile: handleImportFileBytes,
    validateFile: validateImportFile,
    onImportError: handleImportError,
  });

  const handleNewPersona = useCallback(() => {
    openPersonaEditor();
  }, [openPersonaEditor]);

  useEffect(() => {
    const pillCls =
      "h-8 rounded-full bg-[var(--surface-button)] px-3 text-[14px] text-black/70 hover:bg-[var(--surface-button)]/80";
    setTopBarActions(
      <>
        <Button
          type="button"
          variant="ghost"
          className={pillCls}
          onClick={() => void handleImportPicker()}
        >
          <Upload className="mr-2 size-4" />
          {t("common:actions.import")}
        </Button>
        <Button
          type="button"
          variant="ghost"
          className={pillCls}
          onClick={handleNewPersona}
        >
          <Plus className="mr-2 size-4" />
          {t("view.newPersona")}
        </Button>
      </>,
    );
    return () => setTopBarActions(null);
  }, [setTopBarActions, t, handleImportPicker, handleNewPersona]);

  return (
    <div className="flex flex-1 flex-col h-full min-h-0">
      <div className="flex-1 overflow-y-auto min-h-0">
        <div className="max-w-7xl mx-auto w-full px-6 py-8 page-transition">
          <PersonaGallery
            personas={personas}
            onSelectPersona={(p) => openPersonaEditor(p, "details")}
            onEditPersona={(p) => openPersonaEditor(p, "edit")}
            onDuplicatePersona={handleDuplicatePersona}
            onDeletePersona={handleDeletePersona}
            onExportPersona={handleExportPersona}
            isLoading={personasLoading}
            dropHandlers={dropHandlers}
            isDragOver={isDragOver}
          />
        </div>

        <BottomFade />
      </div>

      {/* Persona editor modal */}
      <PersonaEditor
        persona={editingPersona ?? undefined}
        isOpen={personaEditorOpen}
        mode={personaEditorMode}
        onClose={closePersonaEditor}
        onSave={handleSavePersona}
        onDuplicate={handleDuplicatePersona}
        onEdit={(persona) => openPersonaEditor(persona, "edit")}
        onDelete={handleDeletePersona}
      />

      {/* Delete confirmation dialog */}
      <AlertDialog
        open={!!deletingPersona}
        onOpenChange={(open) => !open && setDeletingPersona(null)}
      >
        <AlertDialogContent className="max-w-sm">
          <AlertDialogHeader>
            <AlertDialogTitle>{t("view.deleteTitle")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("view.deleteDescription", {
                name: deletingPersona?.displayName ?? "",
              })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common:actions.cancel")}</AlertDialogCancel>
            <AlertDialogAction
              className={buttonVariants({ variant: "destructive" })}
              onClick={handleConfirmDeletePersona}
            >
              {t("common:actions.delete")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
