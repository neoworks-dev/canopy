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
	    dataUrl: string;
	    truncated: boolean;
	    size: number;
	
	    static createFrom(source: any = {}) {
	        return new Preview(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.path = source["path"];
	        this.kind = source["kind"];
	        this.text = source["text"];
	        this.dataUrl = source["dataUrl"];
	        this.truncated = source["truncated"];
	        this.size = source["size"];
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

export namespace settings {
	
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

