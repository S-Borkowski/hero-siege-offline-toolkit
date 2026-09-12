// Minimal stand-in for Aurie Framework's shared.hpp, so the hs-game-sdk C++
// headers can be compiled and exercised without an Aurie runtime or the game.
//
// Only the surface hs-game-sdk actually touches is declared. Nothing here is
// derived from Aurie's implementation - it is the smallest set of declarations
// that lets our own headers compile and be tested.
//
// IMPORTANT: everything lives in `namespace Aurie`, exactly as it does in the
// real header. An earlier draft of this stub declared AurieStatus and
// AurieSuccess at global scope, which let hooks.hpp compile here while failing
// in ForgePact's real build. A stub that is more permissive than the thing it
// stands in for hides the bugs it was meant to catch.
#pragma once

#include <windows.h>

#include <cstdint>
#include <string_view>

namespace Aurie {

enum AurieStatus : uint32_t {
    AURIE_SUCCESS = 0,
    AURIE_EXTERNAL_ERROR = 1,
    AURIE_OBJECT_NOT_FOUND = 2,
};

constexpr inline bool AurieSuccess(const AurieStatus Status) noexcept {
    return Status == AURIE_SUCCESS;
}

struct AurieModule {
    int placeholder = 0;
};

/// Inline-detour installer. The test harness never reaches this: it injects its
/// own NativeDetourFn, so this only has to exist and link.
inline AurieStatus MmCreateHook(
    AurieModule* Module,
    std::string_view HookIdentifier,
    PVOID SourceFunction,
    PVOID DestinationFunction,
    PVOID* Trampoline
) {
    (void)Module;
    (void)HookIdentifier;
    (void)SourceFunction;
    (void)DestinationFunction;
    if (Trampoline) *Trampoline = nullptr;
    return AURIE_EXTERNAL_ERROR;
}

} // namespace Aurie
