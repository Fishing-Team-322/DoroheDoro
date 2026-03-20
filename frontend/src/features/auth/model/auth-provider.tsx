"use client";

import {
  createContext,
  type ReactNode,
  useEffect,
  useState,
} from "react";
import {
  getCurrentUser,
  isUnauthorizedError,
  login as loginRequest,
  logout as logoutRequest,
  subscribeToUnauthorized,
  updateProfile as updateProfileRequest,
} from "@/src/shared/lib/auth";
import type {
  AuthUser,
  LoginInput,
  UpdateProfileInput,
} from "@/src/shared/lib/auth";

export type AuthStatus = "loading" | "authenticated" | "unauthenticated";

export type AuthContextValue = {
  user: AuthUser | null;
  status: AuthStatus;
  isLoading: boolean;
  login: (input: LoginInput) => Promise<AuthUser>;
  logout: () => Promise<void>;
  refreshUser: () => Promise<AuthUser | null>;
  updateProfile: (input: UpdateProfileInput) => Promise<AuthUser>;
};

export const AuthContext = createContext<AuthContextValue | null>(null);

type AuthProviderProps = {
  children: ReactNode;
};

export function AuthProvider({ children }: AuthProviderProps) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [status, setStatus] = useState<AuthStatus>("loading");

  useEffect(() => {
    let isMounted = true;

    async function loadCurrentUser() {
      try {
        const session = await getCurrentUser();

        if (!isMounted) {
          return;
        }

        setUser(session.user);
        setStatus("authenticated");
      } catch (error) {
        if (!isMounted) {
          return;
        }

        if (!isUnauthorizedError(error)) {
          console.error("Failed to load current user", error);
        }

        setUser(null);
        setStatus("unauthenticated");
      }
    }

    void loadCurrentUser();

    return () => {
      isMounted = false;
    };
  }, []);

  useEffect(() => {
    return subscribeToUnauthorized(() => {
      setUser(null);
      setStatus("unauthenticated");
    });
  }, []);

  async function login(input: LoginInput): Promise<AuthUser> {
    const session = await loginRequest(input);
    setUser(session.user);
    setStatus("authenticated");
    return session.user;
  }

  async function logout(): Promise<void> {
    try {
      await logoutRequest();
    } catch (error) {
      if (!isUnauthorizedError(error)) {
        console.error("Failed to logout", error);
      }
    } finally {
      setUser(null);
      setStatus("unauthenticated");
    }
  }

  async function refreshUser(): Promise<AuthUser | null> {
    setStatus("loading");

    try {
      const session = await getCurrentUser();
      setUser(session.user);
      setStatus("authenticated");
      return session.user;
    } catch (error) {
      if (!isUnauthorizedError(error)) {
        console.error("Failed to refresh current user", error);
      }

      setUser(null);
      setStatus("unauthenticated");
      return null;
    }
  }

  async function updateProfile(input: UpdateProfileInput): Promise<AuthUser> {
    const session = await updateProfileRequest(input);
    setUser(session.user);
    setStatus("authenticated");
    return session.user;
  }

  return (
    <AuthContext.Provider
      value={{
        user,
        status,
        isLoading: status === "loading",
        login,
        logout,
        refreshUser,
        updateProfile,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}
