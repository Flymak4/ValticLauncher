package dev.valtic.cosmetics.data;

public enum CosmeticType {
    WINGS("Wings"),
    CAPE("Capes"),
    HAT("Hats"),
    HORNS("Horns");

    private final String displayName;

    CosmeticType(String displayName) {
        this.displayName = displayName;
    }

    public String getDisplayName() {
        return displayName;
    }
}
