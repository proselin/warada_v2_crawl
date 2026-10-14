
    export type RemoteKeys = 'REMOTE_ALIAS_IDENTIFIER/status-class';
    type PackageType<T> = T extends 'REMOTE_ALIAS_IDENTIFIER/status-class' ? typeof import('REMOTE_ALIAS_IDENTIFIER/status-class') :any;