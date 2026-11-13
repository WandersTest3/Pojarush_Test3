import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

fun secret(env: String, file: String): String =
    System.getenv(env) ?: rootProject.file(file).takeIf { it.isFile }?.readText()?.trim() ?: ""

android {
    testOptions {
        unitTests.isReturnDefaultValues = true
    }
    namespace = "com.wanderstest3.pojarush"
    compileSdk {
        version = release(37) {
            minorApiLevel = 2
        }
    }

    defaultConfig {
        applicationId = "com.wanderstest3.pojarush"
        minSdk = 26
        targetSdk = 34
        versionCode = (project.property("pojarush_version_code") as String).toInt()
        versionName = project.property("pojarush_version_name") as String
        buildConfigField("String", "OAUTH_CLIENT_ID", "\"${secret("OAUTH_CLIENT_ID", ".oauth_client_id.txt")}\"")
    }

    signingConfigs {
        create("release") {
            val store = rootProject.file("pojarush.jks")
            if (store.isFile) {
                storeFile = store
                storePassword = secret("STORE_PASSWORD", ".store_password.txt")
                keyAlias = secret("KEY_ALIAS", ".key_alias.txt").ifEmpty { "pojarush" }
                keyPassword = secret("KEY_PASSWORD", ".key_password.txt")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = signingConfigs.getByName("release")
        }
        debug {
            applicationIdSuffix = ".debug"
            versionNameSuffix = "-debug"
        }
    }

    splits {
        abi {
            isEnable = true
            reset()
            include("arm64-v8a", "armeabi-v7a")
            isUniversalApk = false
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }

    androidResources {
        noCompress += listOf("xz")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }
}

androidComponents {
    onVariants { variant ->
        val base = (project.property("pojarush_version_code") as String).toInt()
        variant.outputs.forEach { output ->
            val abi = output.filters.find { it.filterType == com.android.build.api.variant.FilterConfiguration.FilterType.ABI }?.identifier
            val offset = when (abi) {
                "arm64-v8a" -> 2
                "armeabi-v7a" -> 1
                else -> 0
            }
            output.versionCode.set(base * 10 + offset)
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
    }
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.ui)
    implementation(libs.androidx.ui.graphics)
    implementation(libs.androidx.material3)
    implementation(libs.androidx.appcompat)
    implementation(libs.androidx.games.activity)
    implementation(libs.kotlinx.coroutines.android)
    implementation(libs.okhttp)
    implementation(libs.commons.compress)
    implementation(libs.xz)
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.json:json:20240303")
}
