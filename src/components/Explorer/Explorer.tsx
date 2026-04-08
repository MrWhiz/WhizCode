import React from "react";
import { FileTree } from "./FileTree";
import "./Explorer.css";

interface ExplorerProps {
  path: string;
  onFileOpen: (path: string, name: string) => void;
  onFileDeleted: (path: string) => void;
  onFileRenamed: (oldPath: string, newPath: string) => void;
  refreshKey: number;
  collapseAll: boolean;
  fileFilter: string;
  fileErrors: Record<string, number>;
  gitStatus: {
    branch: string;
    changes: { file: string; status: string }[];
  } | null;
  workspacePath?: string;
}

/**
 * Explorer component for file tree
 *
 * Displays the file tree view
 */
export const Explorer: React.FC<ExplorerProps> = ({
  path,
  onFileOpen,
  onFileDeleted,
  onFileRenamed,
  refreshKey,
  collapseAll,
  fileFilter,
  fileErrors,
  gitStatus,
  workspacePath,
}) => {
  return (
    <div className="explorer-container">
      <FileTree
        path={path}
        onFileOpen={onFileOpen}
        onFileDeleted={onFileDeleted}
        onFileRenamed={onFileRenamed}
        refreshKey={refreshKey}
        collapseAll={collapseAll}
        fileFilter={fileFilter}
        fileErrors={fileErrors}
        gitStatus={gitStatus}
      />
    </div>
  );
};
