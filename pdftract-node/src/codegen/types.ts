/**
 * This file is auto-generated. Do not edit manually.
 */

import { tmpdir } from 'os';
import { join } from 'path';
import { writeFile } from 'fs/promises';

export interface Source {
  toArgs(): string[] | Promise<string[]>;
}

export class PathSource implements Source {
  constructor(private path: string) {}

  toArgs(): string[] {
    return [this.path];
  }
}

export class URLSource implements Source {
  constructor(private url: string) {}

  toArgs(): string[] {
    return [this.url];
  }
}

export class BytesSource implements Source {
  constructor(private bytes: Uint8Array) {}

  async toArgs(): Promise<string[]> {
    const tmp = tmpdir();
    const path = join(tmp, `pdftract-${Date.now()}.pdf`);
    await writeFile(path, this.bytes);
    return [path];
  }
}

export interface Document {
  schema_version: string;
  pages: Page[];
  metadata: Metadata;
  form_fields?: any[];
  errors?: any[];
}

export interface Page {
  page_index: number;
  width: number;
  height: number;
  rotation: number;
  page_type?: string;
  spans: Span[];
  blocks: Block[];
}

export interface Span {
  text: string;
  bbox: [number, number, number, number];
  font: string;
  size: number;
  confidence?: number;
}

export interface Block {
  kind: string;
  text: string;
  bbox: [number, number, number, number];
  level?: number;
}

export interface Match {
  text: string;
  page: number;
  bbox: [number, number, number, number];
  context: {
    before: string;
    after: string;
  };
}

export interface Fingerprint {
  hash: string;
  page_count: number;
  fast_hash: string;
  metadata: Metadata;
}

export interface Classification {
  category: string;
  confidence: number;
  tags: string[];
  heuristics: Record<string, boolean>;
}

export interface Metadata {
  title?: string;
  author?: string;
  subject?: string;
  keywords?: string[];
  creator?: string;
  producer?: string;
  created?: string;
  modified?: string;
  page_count: number;
  is_encrypted?: boolean;
}

export interface ExtractOptions {
  ocrLanguage?: string;
  ocrThreshold?: number;
  preserveLayout?: boolean;
  extractImages?: boolean;
  imageFormat?: string;
  minImageSize?: number;
  password?: string;
}

export interface SearchOptions {
  caseInsensitive?: boolean;
  regex?: boolean;
  wholeWord?: boolean;
  maxResults?: number;
}

export interface BaseOptions {
  timeout?: number;
}

export interface HashOptions extends BaseOptions {}

export interface Receipt {
  fingerprint: string;
  signature: string;
  timestamp: string;
}
