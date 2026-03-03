export interface User {
    id: string; // uuid
    un: string;
    pk: string;
}

export interface LoginResponse {
    id: string;
    un: string;
    pk: string;
    av: string | null;
    encrypted_privkey: string | null;
    privkey_salt: string | null;
    privkey_iv: string | null;
}
