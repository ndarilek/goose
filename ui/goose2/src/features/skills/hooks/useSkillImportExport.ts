import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useFileImportZone } from "@/shared/hooks/useFileImportZone";
import { copyFileToClipboard, saveFileCopy } from "@/shared/api/system";
import { importSkills, type SkillInfo } from "../api/skills";

export function useSkillImportExport(onAfterImport: () => Promise<void>) {
  const { t } = useTranslation(["skills"]);

  const handleCopyFile = async (skill: SkillInfo) => {
    try {
      await copyFileToClipboard(skill.fileLocation);
      toast.success(t("view.fileCopied"));
    } catch {
      toast.error(t("view.copyFileError"));
    }
  };

  const handleSaveCopy = async (skill: SkillInfo) => {
    try {
      const savedPath = await saveFileCopy(skill.fileLocation);
      if (savedPath) {
        toast.success(t("view.copySaved", { path: savedPath }));
      }
    } catch {
      toast.error(t("view.saveCopyError"));
    }
  };

  const handleImport = async (fileBytes: number[], fileName: string) => {
    try {
      await importSkills(fileBytes, fileName);
      await onAfterImport();
      toast.success(t("view.importSuccess"));
    } catch {
      toast.error(t("view.importError"));
    }
  };

  const fileImport = useFileImportZone({ onImportFile: handleImport });

  return { ...fileImport, handleCopyFile, handleSaveCopy };
}
