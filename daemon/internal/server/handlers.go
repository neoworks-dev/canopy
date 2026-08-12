package server

import (
	"context"
	"encoding/json"
	"fmt"

	"canopy/daemon/internal/index"
	"canopy/daemon/proto"
)

func (server *Server) buildHandlers() map[string]handler {
	return map[string]handler{
		proto.MethodHandshake:   server.handleHandshake,
		proto.MethodAddRoot:     server.handleAddRoot,
		proto.MethodRemoveRoot:  server.handleRemoveRoot,
		proto.MethodRoots:       server.handleRoots,
		proto.MethodStatus:      server.handleStatus,
		proto.MethodRescan:      server.handleRescan,
		proto.MethodSearchQuery: server.handleSearch,
		proto.MethodShutdown:    server.handleShutdown,
	}
}

func (server *Server) handleHandshake(_ context.Context, raw json.RawMessage, session *connectionSession) (any, error) {
	params, err := decodeParams[proto.HandshakeParams](raw)
	if err != nil {
		return nil, err
	}
	if params.ProtocolVersion != proto.ProtocolVersion {
		return nil, newError(proto.CodeProtocolMismatch, describeVersionMismatch(params.ProtocolVersion))
	}
	session.handshakeComplete = true
	session.clientName = params.ClientName
	server.logger.Info("client connected", "client", params.ClientName)
	return proto.HandshakeResult{
		DaemonVersion:   server.options.DaemonVersion,
		ProtocolVersion: proto.ProtocolVersion,
		IndexPath:       server.options.IndexPath,
	}, nil
}

func (server *Server) handleAddRoot(_ context.Context, raw json.RawMessage, _ *connectionSession) (any, error) {
	params, err := decodeParams[proto.AddRootParams](raw)
	if err != nil {
		return nil, err
	}
	if err := server.indexer.AddRoot(params.Path); err != nil {
		return nil, err
	}
	return proto.EmptyResult{}, nil
}

func (server *Server) handleRemoveRoot(_ context.Context, raw json.RawMessage, _ *connectionSession) (any, error) {
	params, err := decodeParams[proto.RemoveRootParams](raw)
	if err != nil {
		return nil, err
	}
	if err := server.indexer.RemoveRoot(params.Path); err != nil {
		return nil, err
	}
	return proto.EmptyResult{}, nil
}

func (server *Server) handleRoots(_ context.Context, _ json.RawMessage, _ *connectionSession) (any, error) {
	roots, err := server.indexer.Roots()
	if err != nil {
		return nil, err
	}
	return proto.RootsResult{Roots: toProtocolRoots(roots)}, nil
}

func (server *Server) handleStatus(_ context.Context, _ json.RawMessage, _ *connectionSession) (any, error) {
	status, err := server.indexer.Status()
	if err != nil {
		return nil, err
	}
	return toProtocolStatus(status), nil
}

func (server *Server) handleRescan(_ context.Context, raw json.RawMessage, _ *connectionSession) (any, error) {
	params, err := decodeParams[proto.RescanParams](raw)
	if err != nil {
		return nil, err
	}
	if err := server.indexer.Rescan(params.Path); err != nil {
		return nil, err
	}
	return proto.EmptyResult{}, nil
}

func (server *Server) handleSearch(_ context.Context, raw json.RawMessage, _ *connectionSession) (any, error) {
	params, err := decodeParams[proto.SearchParams](raw)
	if err != nil {
		return nil, err
	}
	records, truncated, err := server.indexer.Search(index.Query{
		Text:            params.Text,
		Limit:           params.Limit,
		DirectoriesOnly: params.DirectoriesOnly,
		Scope:           params.Scope,
	})
	if err != nil {
		return nil, err
	}
	return proto.SearchResult{Hits: toProtocolHits(records), Truncated: truncated}, nil
}

func (server *Server) handleShutdown(_ context.Context, _ json.RawMessage, session *connectionSession) (any, error) {
	server.logger.Info("shutdown requested", "client", session.clientName)
	go server.requestStop()
	return proto.EmptyResult{}, nil
}

// decodeParams treats absent params as the zero value, so methods without
// arguments need no params field on the wire.
func decodeParams[Params any](raw json.RawMessage) (Params, error) {
	var params Params
	if len(raw) == 0 {
		return params, nil
	}
	if err := json.Unmarshal(raw, &params); err != nil {
		return params, newError(proto.CodeInvalidParams, err.Error())
	}
	return params, nil
}

func toProtocolRoots(roots []index.RootRecord) []proto.Root {
	converted := make([]proto.Root, 0, len(roots))
	for _, root := range roots {
		converted = append(converted, proto.Root{
			Path:         root.Path,
			FileCount:    root.FileCount,
			LastScanUnix: root.LastScanUnix,
		})
	}
	return converted
}

func toProtocolHits(records []index.FileRecord) []proto.Hit {
	hits := make([]proto.Hit, 0, len(records))
	for _, record := range records {
		hits = append(hits, proto.Hit{
			Path:        record.Path,
			Name:        record.Name,
			IsDir:       record.IsDir,
			Size:        record.Size,
			ModTimeUnix: record.ModTimeUnix,
		})
	}
	return hits
}

func toProtocolStatus(status index.Status) proto.StatusResult {
	return proto.StatusResult{
		State:              describeState(status.Scanning),
		FilesIndexed:       status.FilesIndexed,
		ScanningPath:       status.ScanningPath,
		WatchedDirectories: status.WatchedDirectories,
		WatchLimitReached:  status.WatchLimitReached,
		LastScanUnix:       status.LastScanUnix,
	}
}

func describeState(scanning bool) string {
	if scanning {
		return proto.StateScanning
	}
	return proto.StateIdle
}

func describeVersionMismatch(clientVersion int) string {
	return fmt.Sprintf("client speaks protocol %d, daemon speaks %d", clientVersion, proto.ProtocolVersion)
}
