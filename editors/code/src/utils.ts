import * as url from "node:url"

import {
    Location as LspLocation,
    Range as LspRange,
    Position as LspPosition,
    Diagnostic as LspDiagnostic,
} from 'vscode-languageserver/node';

export interface Position {
    line: number;
    column: number;
}

export function toLspPosition(position: Position): LspPosition {
    return {
        line: position.line - 1,
        character: position.column - 1
    };
}

export interface Range {
    start: Position;
    end: Position;
}

export function toLspRange(range: Range): LspRange {
    return {
        start: toLspPosition(range.start),
        end: toLspPosition(range.end)
    };
}

export interface Location {
    path: string;
    range: Range;
}

export function toLspLocation(location: Location): LspLocation | null {
    if (location == null) {
        return null;
    }
    return {
        uri: pathToUri(location.path),
        range: toLspRange(location.range)
    };
}

export interface Error {
    range: Range;
    message: string;
}

export function toLspDiagnostic(error: Error): LspDiagnostic {
    return {
        range: toLspRange(error.range),
        message: error.message
    };
}

export function uriToPath(uri: string): string {
    return url.fileURLToPath(uri);
}

export function pathToUri(path: string): string {
    return url.pathToFileURL(path).toString();
}
