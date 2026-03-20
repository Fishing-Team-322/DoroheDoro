export type AuthUser = {
  id: string;
  email: string;
  login: string;
  displayName: string;
  updatedAt?: string;
};

export type LoginInput = {
  identifier: string;
  password: string;
};

export type UpdateProfileInput = {
  displayName: string;
};

export type SessionPayload = {
  user: AuthUser;
  csrfToken?: string;
  expiresAt?: string;
};

export type LogoutPayload = {
  success: boolean;
};
