import { invoke } from "@neoworks/svelte-gpui";
import type { archive, bookmarks, fileops, filesystem, git, launcher, recents, searches, settings, thumbnail, zoxide } from "./models";

export function AddBookmark(path: string): Promise<void> {
	return invoke<void>("AddBookmark", { path });
}

export function AddTag(paths: Array<string>, name: string): Promise<void> {
	return invoke<void>("AddTag", { paths, name });
}

export function ApplicationsFor(path: string): Promise<Array<launcher.Application>> {
	return invoke<Array<launcher.Application>>("ApplicationsFor", { path });
}

export function Bookmarks(): Promise<Array<bookmarks.Bookmark>> {
	return invoke<Array<bookmarks.Bookmark>>("Bookmarks");
}

export function CancelDuplicateSearch(searchId: string): Promise<void> {
	return invoke<void>("CancelDuplicateSearch", { searchId });
}

export function CancelMeasurement(measurementId: string): Promise<void> {
	return invoke<void>("CancelMeasurement", { measurementId });
}

export function CancelOperation(operationId: string): Promise<void> {
	return invoke<void>("CancelOperation", { operationId });
}

export function ClearRecents(): Promise<void> {
	return invoke<void>("ClearRecents");
}

export function CopyPathsToClipboard(paths: Array<string>): Promise<void> {
	return invoke<void>("CopyPathsToClipboard", { paths });
}

export function CreateFile(directory: string, name: string): Promise<string> {
	return invoke<string>("CreateFile", { directory, name });
}

export function CreateFolder(directory: string, name: string): Promise<string> {
	return invoke<string>("CreateFolder", { directory, name });
}

export function Devices(): Promise<Array<filesystem.Device>> {
	return invoke<Array<filesystem.Device>>("Devices");
}

export function EmptyTrash(): Promise<void> {
	return invoke<void>("EmptyTrash");
}

export function ExtractionFolder(archivePath: string): Promise<string> {
	return invoke<string>("ExtractionFolder", { archivePath });
}

export function FilesWithTag(name: string): Promise<Array<filesystem.Entry>> {
	return invoke<Array<filesystem.Entry>>("FilesWithTag", { name });
}

export function FindDuplicates(root: string): Promise<string> {
	return invoke<string>("FindDuplicates", { root });
}

export function ForgetSearch(name: string): Promise<void> {
	return invoke<void>("ForgetSearch", { name });
}

export function ForgetTag(name: string): Promise<void> {
	return invoke<void>("ForgetTag", { name });
}

export function GitReport(path: string): Promise<git.Report> {
	return invoke<git.Report>("GitReport", { path });
}

export function HomeDirectory(): Promise<string> {
	return invoke<string>("HomeDirectory");
}

export function KnownTags(): Promise<Array<string>> {
	return invoke<Array<string>>("KnownTags");
}

export function ListArchive(archivePath: string): Promise<Array<archive.Member>> {
	return invoke<Array<archive.Member>>("ListArchive", { archivePath });
}

export function ListDirectory(path: string, order: filesystem.Order): Promise<filesystem.Listing> {
	return invoke<filesystem.Listing>("ListDirectory", { path, order });
}

export function ListTrash(): Promise<Array<fileops.TrashedItem>> {
	return invoke<Array<fileops.TrashedItem>>("ListTrash");
}

export function LoadSettings(): Promise<settings.Settings> {
	return invoke<settings.Settings>("LoadSettings");
}

export function MeasureFolder(path: string): Promise<string> {
	return invoke<string>("MeasureFolder", { path });
}

export function OpenPath(path: string): Promise<void> {
	return invoke<void>("OpenPath", { path });
}

export function OpenTerminal(directory: string): Promise<void> {
	return invoke<void>("OpenTerminal", { directory });
}

export function OpenWith(path: string, applicationId: string): Promise<void> {
	return invoke<void>("OpenWith", { path, applicationId });
}

export function PauseOperation(operationId: string): Promise<void> {
	return invoke<void>("PauseOperation", { operationId });
}

export function Places(): Promise<Array<filesystem.Place>> {
	return invoke<Array<filesystem.Place>>("Places");
}

export function PreviewFile(path: string): Promise<filesystem.Preview> {
	return invoke<filesystem.Preview>("PreviewFile", { path });
}

export function ReadClipboardPaths(): Promise<Array<string>> {
	return invoke<Array<string>>("ReadClipboardPaths");
}

export function RecentEntries(): Promise<Array<recents.Entry>> {
	return invoke<Array<recents.Entry>>("RecentEntries");
}

export function Redo(): Promise<string> {
	return invoke<string>("Redo");
}

export function RedoLabel(): Promise<string> {
	return invoke<string>("RedoLabel");
}

export function RemoveBookmark(path: string): Promise<void> {
	return invoke<void>("RemoveBookmark", { path });
}

export function RemoveTag(paths: Array<string>, name: string): Promise<void> {
	return invoke<void>("RemoveTag", { paths, name });
}

export function RenameEntry(path: string, newName: string): Promise<string> {
	return invoke<string>("RenameEntry", { path, newName });
}

export function RenameSearch(name: string, newName: string): Promise<void> {
	return invoke<void>("RenameSearch", { name, newName });
}

export function ResolveConflict(operationId: string, resolution: string, applyToAll: boolean): Promise<void> {
	return invoke<void>("ResolveConflict", { operationId, resolution, applyToAll });
}

export function RestoreFromTrash(trashPath: string): Promise<string> {
	return invoke<string>("RestoreFromTrash", { trashPath });
}

export function ResumeOperation(operationId: string): Promise<void> {
	return invoke<void>("ResumeOperation", { operationId });
}

export function SaveSearch(name: string, query: string, scope: string): Promise<void> {
	return invoke<void>("SaveSearch", { name, query, scope });
}

export function SaveSettings(next: settings.ViewSettings): Promise<void> {
	return invoke<void>("SaveSettings", { next });
}

export function SavedSearches(): Promise<Array<searches.Search>> {
	return invoke<Array<searches.Search>>("SavedSearches");
}

export function SetDefaultApplication(path: string, applicationId: string): Promise<void> {
	return invoke<void>("SetDefaultApplication", { path, applicationId });
}

export function StartOperation(kind: string, sources: Array<string>, destination: string): Promise<string> {
	return invoke<string>("StartOperation", { kind, sources, destination });
}

export function StopWatching(): Promise<void> {
	return invoke<void>("StopWatching");
}

export function SuggestName(directory: string, name: string): Promise<string> {
	return invoke<string>("SuggestName", { directory, name });
}

export function TagsOf(path: string): Promise<Array<string>> {
	return invoke<Array<string>>("TagsOf", { path });
}

export function ThumbnailFor(path: string): Promise<thumbnail.Thumbnail> {
	return invoke<thumbnail.Thumbnail>("ThumbnailFor", { path });
}

export function Undo(): Promise<string> {
	return invoke<string>("Undo");
}

export function UndoLabel(): Promise<string> {
	return invoke<string>("UndoLabel");
}

export function ValidateEntryName(name: string): Promise<void> {
	return invoke<void>("ValidateEntryName", { name });
}

export function WatchDirectories(paths: Array<string>): Promise<void> {
	return invoke<void>("WatchDirectories", { paths });
}

export function ZoxideAvailable(): Promise<boolean> {
	return invoke<boolean>("ZoxideAvailable");
}

export function ZoxideQuery(query: string, exclude: string): Promise<Array<zoxide.Entry>> {
	return invoke<Array<zoxide.Entry>>("ZoxideQuery", { query, exclude });
}

export function RememberWindowSize(width: number, height: number): Promise<void> {
	return invoke<void>("RememberWindowSize", { width, height });
}
