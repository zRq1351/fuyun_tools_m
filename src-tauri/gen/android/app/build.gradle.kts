import java.util.Properties
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

// Release 签名配置（keystore.properties 不存在时跳过，debug 构建不受影响）
// keystore.properties 已加入 .gitignore，切勿提交
// 注意：keystore.properties 位于 android/ 根目录，不在 app/ 模块目录下，
// 必须用 rootProject.file 定位，否则 hasKeystore 恒为 false，产物始终是 unsigned。
val keystoreProperties = Properties().apply {
    val propFile = rootProject.file("keystore.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}
val hasKeystore = keystoreProperties.getProperty("storeFile") != null

android {
    compileSdk = 37
    namespace = "com.fuyun.clipboard"
    defaultConfig {
        // 局域网同步服务走明文 HTTP；release 同样放行 cleartext
        // （此前仅 debug 放开，release 为 false，虽不影响纯 Rust reqwest，
        //  但为 WebView/插件层面的任何 http 请求排除嫌疑）
        manifestPlaceholders["usesCleartextTraffic"] = "true"
        applicationId = "com.fuyun.clipboard"
        minSdk = 24
        targetSdk = 37
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")
    }
    signingConfigs {
        if (hasKeystore) {
            create("release") {
                storeFile = file(keystoreProperties.getProperty("storeFile"))
                storePassword = keystoreProperties.getProperty("storePassword")
                keyAlias = keystoreProperties.getProperty("keyAlias")
                keyPassword = keystoreProperties.getProperty("keyPassword")
            }
        }
    }
    buildTypes {
        getByName("debug") {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {
                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                jniLibs.keepDebugSymbols.add("*/x86/*.so")
                jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
            }
        }
        getByName("release") {
            optimization {
               enable = true
            }
            if (hasKeystore) {
                signingConfig = signingConfigs.getByName("release")
            }
            proguardFiles(
                *fileTree(".") {
                  include("**/*.pro")
                  exclude("build/**")
                }.files.toTypedArray()
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    buildFeatures {
        buildConfig = true
    }
}

kotlin {
    compilerOptions {
        jvmTarget = JvmTarget.JVM_1_8
    }
}

rust {
    rootDirRel = "../../../"
}

// barcode-scanner 默认依赖 play-services-mlkit（unbundled），模型需从 Google Play 服务下载，
// 国内机型（无 GMS）会拿到 null 直接崩。改为 bundled 版：模型打进 APK，离线可用。
configurations.configureEach {
    exclude(group = "com.google.android.gms", module = "play-services-mlkit-barcode-scanning")
}

dependencies {
    implementation("com.google.mlkit:barcode-scanning:17.3.0")
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.lifecycle:lifecycle-process:2.10.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = file("tauri.build.gradle.kts"))
