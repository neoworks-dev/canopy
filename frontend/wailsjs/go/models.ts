export namespace archive {
	
	export class Member {
	    name: string;
	    size: number;
	    isDir: boolean;
	    modTime: string;
	    mode: string;
	
	    static createFrom(source: any = {}) {
	        return new Member(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.size = source["size"];
	        this.isDir = source["isDir"];
	        this.modTime = source["modTime"];
	        this.mode = source["mode"];
	    }
	}

}

export namespace audio {
	
	export class Details {
	    duration: number;
	    bitrate: number;
	    channels: number;
	    sampleRate: number;
	    codec: string;
	    title: string;
	    artist: string;
	    album: string;
	    waveform: number[];
	
	    static createFrom(source: any = {}) {
	        return new Details(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.duration = source["duration"];
	        this.bitrate = source["bitrate"];
	        this.channels = source["channels"];
	        this.sampleRate = source["sampleRate"];
	        this.codec = source["codec"];
	        this.title = source["title"];
	        this.artist = source["artist"];
	        this.album = source["album"];
	        this.waveform = source["waveform"];
	    }
	}

}

export namespace bookmarks {
	
	export class Bookmark {
	    name: string;
	    path: string;
	
	    static createFrom(source: any = {}) {
	        return new Bookmark(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.path = source["path"];
	    }
	}

}

export namespace fileops {
	
	export class TrashedItem {
	    name: string;
	    trashPath: string;
	    originalPath: string;
	    deletedAt: string;
	    size: number;
	    isDir: boolean;
	
	    static createFrom(source: any = {}) {
	        return new TrashedItem(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.trashPath = source["trashPath"];
	        this.originalPath = source["originalPath"];
	        this.deletedAt = source["deletedAt"];
	        this.size = source["size"];
	        this.isDir = source["isDir"];
	    }
	}

}

export namespace filesystem {
	
	export class Device {
	    name: string;
	    path: string;
	    isRemovable: boolean;
	    totalBytes: number;
	    freeBytes: number;
	
	    static createFrom(source: any = {}) {
	        return new Device(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.path = source["path"];
	        this.isRemovable = source["isRemovable"];
	        this.totalBytes = source["totalBytes"];
	        this.freeBytes = source["freeBytes"];
	    }
	}
	export class Entry {
	    name: string;
	    path: string;
	    isDir: boolean;
	    size: number;
	    modTime: string;
	    mode: string;
	
	    static createFrom(source: any = {}) {
	        return new Entry(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.path = source["path"];
	        this.isDir = source["isDir"];
	        this.size = source["size"];
	        this.modTime = source["modTime"];
	        this.mode = source["mode"];
	    }
	}
	export class Listing {
	    path: string;
	    parentPath: string;
	    entries: Entry[];
	
	    static createFrom(source: any = {}) {
	        return new Listing(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.path = source["path"];
	        this.parentPath = source["parentPath"];
	        this.entries = this.convertValues(source["entries"], Entry);
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}
	export class Order {
	    key: string;
	    descending: boolean;
	    directoriesFirst: boolean;
	
	    static createFrom(source: any = {}) {
	        return new Order(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.key = source["key"];
	        this.descending = source["descending"];
	        this.directoriesFirst = source["directoriesFirst"];
	    }
	}
	export class Place {
	    name: string;
	    path: string;
	    kind: string;
	
	    static createFrom(source: any = {}) {
	        return new Place(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.path = source["path"];
	        this.kind = source["kind"];
	    }
	}
	export class Preview {
	    path: string;
	    kind: string;
	    text: string;
	    tokens: highlight.Token[];
	    dataUrl: string;
	    truncated: boolean;
	    size: number;
	    audio: audio.Details;
	
	    static createFrom(source: any = {}) {
	        return new Preview(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.path = source["path"];
	        this.kind = source["kind"];
	        this.text = source["text"];
	        this.tokens = this.convertValues(source["tokens"], highlight.Token);
	        this.dataUrl = source["dataUrl"];
	        this.truncated = source["truncated"];
	        this.size = source["size"];
	        this.audio = this.convertValues(source["audio"], audio.Details);
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}

}

export namespace git {
	
	export class Repository {
	    root: string;
	    branch: string;
	
	    static createFrom(source: any = {}) {
	        return new Repository(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.root = source["root"];
	        this.branch = source["branch"];
	    }
	}
	export class Report {
	    repository: Repository;
	    isRepo: boolean;
	    statuses: Record<string, string>;
	
	    static createFrom(source: any = {}) {
	        return new Report(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.repository = this.convertValues(source["repository"], Repository);
	        this.isRepo = source["isRepo"];
	        this.statuses = source["statuses"];
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}

}

export namespace highlight {
	
	export class Token {
	    class: string;
	    text: string;
	
	    static createFrom(source: any = {}) {
	        return new Token(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.class = source["class"];
	        this.text = source["text"];
	    }
	}

}

export namespace launcher {
	
	export class Application {
	    id: string;
	    name: string;
	    icon: string;
	    exec: string;
	
	    static createFrom(source: any = {}) {
	        return new Application(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.id = source["id"];
	        this.name = source["name"];
	        this.icon = source["icon"];
	        this.exec = source["exec"];
	    }
	}

}

export namespace proto {
	
	export class Hit {
	    path: string;
	    name: string;
	    isDir: boolean;
	    size: number;
	    modTimeUnix: number;
	
	    static createFrom(source: any = {}) {
	        return new Hit(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.path = source["path"];
	        this.name = source["name"];
	        this.isDir = source["isDir"];
	        this.size = source["size"];
	        this.modTimeUnix = source["modTimeUnix"];
	    }
	}
	export class SearchResult {
	    hits: Hit[];
	    truncated: boolean;
	
	    static createFrom(source: any = {}) {
	        return new SearchResult(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.hits = this.convertValues(source["hits"], Hit);
	        this.truncated = source["truncated"];
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}
	export class StatusResult {
	    state: string;
	    filesIndexed: number;
	    scanningPath: string;
	    watchedDirectories: number;
	    watchLimitReached: boolean;
	    lastScanUnix: number;
	
	    static createFrom(source: any = {}) {
	        return new StatusResult(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.state = source["state"];
	        this.filesIndexed = source["filesIndexed"];
	        this.scanningPath = source["scanningPath"];
	        this.watchedDirectories = source["watchedDirectories"];
	        this.watchLimitReached = source["watchLimitReached"];
	        this.lastScanUnix = source["lastScanUnix"];
	    }
	}

}

export namespace recents {
	
	export class Entry {
	    name: string;
	    path: string;
	    isDir: boolean;
	    openedAt: string;
	
	    static createFrom(source: any = {}) {
	        return new Entry(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.path = source["path"];
	        this.isDir = source["isDir"];
	        this.openedAt = source["openedAt"];
	    }
	}

}

export namespace searches {
	
	export class Search {
	    name: string;
	    query: string;
	    scope: string;
	
	    static createFrom(source: any = {}) {
	        return new Search(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.name = source["name"];
	        this.query = source["query"];
	        this.scope = source["scope"];
	    }
	}

}

export namespace settings {
	
	export class Keybinding {
	    key: string;
	    command: string;
	    when?: string;
	
	    static createFrom(source: any = {}) {
	        return new Keybinding(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.key = source["key"];
	        this.command = source["command"];
	        this.when = source["when"];
	    }
	}
	export class Panes {
	    sidebar: number;
	    details: number;
	
	    static createFrom(source: any = {}) {
	        return new Panes(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.sidebar = source["sidebar"];
	        this.details = source["details"];
	    }
	}
	export class Window {
	    width: number;
	    height: number;
	    x: number;
	    y: number;
	
	    static createFrom(source: any = {}) {
	        return new Window(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.width = source["width"];
	        this.height = source["height"];
	        this.x = source["x"];
	        this.y = source["y"];
	    }
	}
	export class Settings {
	    version: number;
	    viewMode: string;
	    columns: string[];
	    showHidden: boolean;
	    sort: filesystem.Order;
	    lastDirectory: string;
	    window: Window;
	    panes: Panes;
	    collapsedSections?: string[];
	    alwaysShowDetails: boolean;
	    keybindings?: Keybinding[];
	
	    static createFrom(source: any = {}) {
	        return new Settings(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.version = source["version"];
	        this.viewMode = source["viewMode"];
	        this.columns = source["columns"];
	        this.showHidden = source["showHidden"];
	        this.sort = this.convertValues(source["sort"], filesystem.Order);
	        this.lastDirectory = source["lastDirectory"];
	        this.window = this.convertValues(source["window"], Window);
	        this.panes = this.convertValues(source["panes"], Panes);
	        this.collapsedSections = source["collapsedSections"];
	        this.alwaysShowDetails = source["alwaysShowDetails"];
	        this.keybindings = this.convertValues(source["keybindings"], Keybinding);
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}

}

export namespace thumbnail {
	
	export class Thumbnail {
	    path: string;
	    dataUrl: string;
	
	    static createFrom(source: any = {}) {
	        return new Thumbnail(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.path = source["path"];
	        this.dataUrl = source["dataUrl"];
	    }
	}

}

export namespace zoxide {
	
	export class Entry {
	    path: string;
	    score: number;
	
	    static createFrom(source: any = {}) {
	        return new Entry(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.path = source["path"];
	        this.score = source["score"];
	    }
	}

}

